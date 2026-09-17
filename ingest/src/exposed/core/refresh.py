"""Refresh Commons membership through application-owned source and storage ports."""

import logging
from collections.abc import Iterable, Iterator
from datetime import date

from exposed.core.errors import ImportFailed, ImportValidationError, safe_error
from exposed.core.models import CommonsService, Member, MemberHistory, MemberProfile
from exposed.core.ports import MemberSource, MemberWriter, RefreshStore

logger = logging.getLogger(__name__)


def match_histories[T: MemberHistory](member_ids: set[int], batch: tuple[T, ...]) -> dict[int, T]:
    """Match a history batch to its requested members before indexing it by ID."""
    histories: dict[int, T] = {}
    for history in batch:
        member_id = history.parliament_member_id
        if member_id in histories:
            raise ImportValidationError(f"Duplicate history for member {member_id}")
        histories[member_id] = history
    if histories.keys() != member_ids:
        raise ImportValidationError("History response does not contain all requested member IDs")
    return histories


def _unique_profiles(
    profiles: Iterable[MemberProfile], seen: dict[int, MemberProfile]
) -> Iterator[MemberProfile]:
    """Validate repeated identities within one source stream, including across pages."""
    for profile in profiles:
        member_id = profile.parliament_member_id
        if member_id in seen:
            if seen[member_id] != profile:
                raise ImportValidationError(f"Conflicting duplicate member {member_id}")
            logger.warning("Duplicate member %s; identical profile collapsed", member_id)
            continue
        seen[member_id] = profile
        yield profile


def _import_batches(
    writer: MemberWriter,
    term_start: date,
    as_of: date,
    source: MemberSource,
) -> dict[str, int]:
    """Write each page immediately; the caller commits only after every page is complete."""
    summary = dict.fromkeys(
        [
            "members",
            "current_commons",
            "former_commons",
            "inserted",
            "updated",
            "unchanged",
            "service_periods",
            "excluded_candidates",
        ],
        0,
    )

    logger.info("Fetching current Commons member IDs")
    current_ids = {
        member.parliament_member_id for member in _unique_profiles(source.current_commons(), {})
    }

    logger.info("Importing Commons service since %s in batches", term_start)
    seen_profiles: dict[int, MemberProfile] = {}
    for batch in source.commons_candidates(term_start, as_of):
        profiles = tuple(_unique_profiles(batch, seen_profiles))
        if not profiles:
            continue
        member_ids = {member.parliament_member_id for member in profiles}
        histories = match_histories(member_ids, source.member_histories(member_ids))
        for profile in profiles:
            member_id = profile.parliament_member_id
            current = member_id in current_ids
            service = CommonsService.from_history(
                histories[member_id],
                term_start=term_start,
                as_of=as_of,
                is_current_commons=current,
            )
            if not service.periods:
                summary["excluded_candidates"] += 1
                continue
            member = Member.from_profile(profile, is_current_commons=current)
            write_result = writer.write_member(member, service.periods)

            summary[write_result] += 1
            summary["members"] += 1
            summary["current_commons"] += current
            summary["former_commons"] += not current
            summary["service_periods"] += len(service.periods)

        logger.info("Processed %s historical candidates (uncommitted)", len(seen_profiles))

    if not current_ids <= seen_profiles.keys():
        raise ImportValidationError(
            "Current search contains members missing from historical search"
        )
    if not summary["members"]:
        raise ImportValidationError("No Commons service was found for the configured term")
    return summary


def refresh_members(
    term_start: date,
    as_of: date,
    source: MemberSource,
    store: RefreshStore,
) -> dict[str, object]:
    """Refresh a term on a fixed date, publishing all batches together."""
    try:
        if term_start > as_of:
            raise ImportValidationError("Term start cannot be after the import date")
        with store.refresh(term_start) as writer:
            summary = _import_batches(writer, term_start, as_of, source)
        return {
            "status": "succeeded",
            "term_start": term_start.isoformat(),
            "as_of": as_of.isoformat(),
            **summary,
        }
    except BaseException as exc:
        raise ImportFailed(safe_error(exc), interrupted=isinstance(exc, KeyboardInterrupt)) from exc
