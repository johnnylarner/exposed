"""Import every batch in the same database transaction."""

import logging
from collections.abc import Iterator
from datetime import date, datetime
from zoneinfo import ZoneInfo

import psycopg
from psycopg.rows import DictRow, dict_row
from pydantic import ValidationError

from exposed.api import APIError, MembersAPI
from exposed.db import DatabaseConnection, ensure_term, write_member
from exposed.models import (
    CommonsService,
    ImportValidationError,
    Member,
    MemberHistory,
    SearchPage,
    validation_error_message,
)

BATCH_SIZE = 100
logger = logging.getLogger(__name__)


class ImportFailed(RuntimeError):
    def __init__(self, message: str, *, interrupted: bool = False):
        super().__init__(message)
        self.interrupted = interrupted


def connect(database_url: str) -> DatabaseConnection:
    return psycopg.Connection[DictRow].connect(
        database_url,
        autocommit=True,
        row_factory=dict_row,
        connect_timeout=10,
        application_name="exposed",
    )


def safe_error(exc: BaseException) -> str:
    if isinstance(exc, (ImportValidationError, APIError)):
        return str(exc)
    if isinstance(exc, ValidationError):
        return validation_error_message(exc)
    if isinstance(exc, KeyboardInterrupt):
        return "Import interrupted"
    if isinstance(exc, psycopg.Error):
        # DSNs and server error details can contain credentials; never emit them in job logs.
        code = exc.sqlstate or "unknown"
        return f"Database operation failed ({type(exc).__name__}, SQLSTATE {code})"
    return f"Unexpected importer failure ({type(exc).__name__})"


def search_pages(api: MembersAPI, filters: dict[str, str | int]) -> Iterator[SearchPage]:
    """Advance by the number returned until the API's reported total is reached."""
    offset = 0
    while True:
        page = api.search_page(filters, skip=offset, take=BATCH_SIZE)
        if not page.members and offset < page.total_results:
            raise ImportValidationError("Search ended before all advertised members were returned")
        yield page
        offset += len(page.members)
        if offset >= page.total_results:
            return


def load_histories(api: MembersAPI, member_ids: set[int]) -> dict[int, MemberHistory]:
    """Match a history batch to its requested members before indexing it by ID."""
    if not 1 <= len(member_ids) <= BATCH_SIZE:
        raise ImportValidationError(f"History requests need between 1 and {BATCH_SIZE} member IDs")
    batch = api.histories(member_ids)
    histories: dict[int, MemberHistory] = {}
    for history in batch.histories:
        member_id = history.parliament_member_id
        if member_id in histories:
            raise ImportValidationError(f"Duplicate history for member {member_id}")
        histories[member_id] = history
    if histories.keys() != member_ids:
        raise ImportValidationError("History response does not contain all requested member IDs")
    return histories


def _import_batches(
    conn: DatabaseConnection,
    term_start: date,
    as_of: date,
    api: MembersAPI,
) -> dict[str, int]:
    """Write each page immediately; the caller commits only after every page is complete."""
    term_id = ensure_term(conn, term_start)
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
    current_ids: set[int] = set()
    for page in search_pages(api, {"House": 1, "IsCurrentMember": "true"}):
        current_ids.update(member.parliament_member_id for member in page.members)

    logger.info("Importing Commons service since %s in batches", term_start)
    seen_ids: set[int] = set()
    for page in search_pages(
        api,
        {
            "MembershipInDateRange.WasMemberOnOrAfter": term_start.isoformat(),
            "MembershipInDateRange.WasMemberOnOrBefore": as_of.isoformat(),
            "MembershipInDateRange.WasMemberOfHouse": 1,
        },
    ):
        if not page.members:
            continue
        histories = load_histories(api, {member.parliament_member_id for member in page.members})
        for profile in page.members:
            member_id = profile.parliament_member_id
            current = member_id in current_ids
            service = CommonsService.from_history(
                histories[member_id],
                term_start=term_start,
                as_of=as_of,
                is_current_commons=current,
            )
            seen_ids.add(member_id)
            if not service.periods:
                summary["excluded_candidates"] += 1
                continue
            member = Member.from_profile(profile, is_current_commons=current)
            write_result = write_member(conn, term_id, member, service.periods)

            summary[write_result] += 1
            summary["members"] += 1
            summary["current_commons"] += current
            summary["former_commons"] += not current
            summary["service_periods"] += len(service.periods)

        logger.info("Processed %s historical candidates (uncommitted)", len(seen_ids))

    if not current_ids <= seen_ids:
        raise ImportValidationError(
            "Current search contains members missing from historical search"
        )
    if not summary["members"]:
        raise ImportValidationError("No Commons service was found for the configured term")
    return summary


def run_import(
    database_url: str,
    term_start: date,
    api: MembersAPI,
    *,
    as_of: date | None = None,
) -> dict[str, object]:
    as_of = as_of or datetime.now(ZoneInfo("Europe/London")).date()
    with connect(database_url) as conn:
        try:
            if term_start > as_of:
                raise ImportValidationError("Term start cannot be after the import date")
            with conn.transaction():
                summary = _import_batches(conn, term_start, as_of, api)
            return {
                "status": "succeeded",
                "term_start": term_start.isoformat(),
                "as_of": as_of.isoformat(),
                **summary,
            }
        except BaseException as exc:
            raise ImportFailed(
                safe_error(exc), interrupted=isinstance(exc, KeyboardInterrupt)
            ) from exc
