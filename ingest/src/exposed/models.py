"""Validate source data and derive dated Commons service for the configured term."""

from dataclasses import dataclass
from datetime import date, datetime
from typing import Any


class ImportValidationError(ValueError):
    """The source is incomplete, inconsistent, or unsuitable for this import."""


def object_value(value: object, context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ImportValidationError(f"{context}: expected an object")
    return value


def integer(value: object, context: str, minimum: int = 1) -> int:
    if type(value) is not int or value < minimum:
        raise ImportValidationError(f"{context}: expected an integer >= {minimum}")
    return value


def source_date(value: object, context: str) -> date:
    try:
        if not isinstance(value, str):
            raise ValueError
        return datetime.fromisoformat(value).date()
    except ValueError as exc:
        raise ImportValidationError(f"{context}: missing or invalid date") from exc


def optional_text(value: object, context: str) -> str | None:
    if value is not None and not isinstance(value, str):
        raise ImportValidationError(f"{context}: expected text or null")
    return value


@dataclass(frozen=True)
class MemberProfile:
    """Latest Parliament profile, independent of current Commons membership."""

    parliament_member_id: int
    name: str
    party_id: int | None
    party_name: str | None
    latest_house: int
    latest_membership_from: str | None


@dataclass(frozen=True)
class Member(MemberProfile):
    is_current_commons: bool


@dataclass(frozen=True)
class HouseMembership:
    house: int
    start_date: date
    end_date: date | None


@dataclass(frozen=True)
class MemberHistory:
    """Dated House memberships for one Parliament member."""

    parliament_member_id: int
    house_memberships: tuple[HouseMembership, ...]


# Both lookups are keyed by Parliament member ID.
type MemberSearchPage = dict[int, MemberProfile]
type MemberHistories = dict[int, MemberHistory]


@dataclass(frozen=True)
class ServicePeriod:
    parliament_member_id: int
    source_start_date: date
    source_end_date: date | None
    served_from: date
    served_until: date | None
    house: int = 1


def parse_profile(value: dict[str, Any]) -> MemberProfile:
    member_id = integer(value.get("id"), "member ID")
    name = value.get("nameDisplayAs")
    if not isinstance(name, str) or not name.strip():
        raise ImportValidationError(f"Member {member_id}: missing display name")
    party = object_value(value.get("latestParty") or {}, f"Member {member_id} party")
    membership = object_value(value.get("latestHouseMembership"), "latest membership")
    house = integer(membership.get("house"), "latest House")
    if house not in (1, 2):
        raise ImportValidationError(f"Member {member_id}: inconsistent latest House")
    party_id = party.get("id")
    return MemberProfile(
        parliament_member_id=member_id,
        name=name,
        party_id=None if party_id is None else integer(party_id, "party ID"),
        party_name=optional_text(party.get("name"), "party name"),
        latest_house=house,
        latest_membership_from=optional_text(membership.get("membershipFrom"), "membership from"),
    )


def parse_member(profile: MemberProfile, current: bool) -> Member:
    if current and profile.latest_house != 1:
        raise ImportValidationError(
            f"Member {profile.parliament_member_id}: inconsistent latest House"
        )
    return Member(
        parliament_member_id=profile.parliament_member_id,
        name=profile.name,
        party_id=profile.party_id,
        party_name=profile.party_name,
        latest_house=profile.latest_house,
        latest_membership_from=profile.latest_membership_from,
        is_current_commons=current,
    )


def parse_history(value: dict[str, Any]) -> MemberHistory:
    member_id = integer(value.get("id"), "history member ID")
    entries = value.get("houseMembershipHistory")
    if not isinstance(entries, list) or not entries:
        raise ImportValidationError(f"Member {member_id}: missing membership history")
    memberships = []
    for entry in entries:
        entry = object_value(entry, "membership history entry")
        house = integer(entry.get("house"), "history House")
        if house not in (1, 2):
            raise ImportValidationError(f"Member {member_id}: unknown history House")
        start = source_date(entry.get("membershipStartDate"), "service start")
        end_value = entry.get("membershipEndDate")
        end = None if end_value is None else source_date(end_value, "service end")
        memberships.append(HouseMembership(house, start, end))
    return MemberHistory(member_id, tuple(memberships))


def service_periods(
    history: MemberHistory,
    term_start: date,
    as_of: date,
    current: bool,
) -> list[ServicePeriod]:
    """Keep and validate each Commons service period in this term."""
    member_id = history.parliament_member_id
    selected: dict[date, ServicePeriod] = {}
    for membership in history.house_memberships:
        if membership.house != 1:
            continue
        start = membership.start_date
        end = membership.end_date
        # An end date is the date service ceased. No invented dissolution dates.
        if start > as_of or (end is not None and end <= term_start):
            continue
        if end is not None and end < start:
            raise ImportValidationError(f"Member {member_id}: service ends before it starts")
        period = ServicePeriod(member_id, start, end, max(start, term_start), end)
        if start in selected and selected[start] != period:
            raise ImportValidationError(f"Member {member_id}: conflicting service periods")
        selected[start] = period
    periods = sorted(selected.values(), key=lambda p: p.source_start_date)
    if not periods:
        if current:
            raise ImportValidationError(f"Current member {member_id} has no service in this term")
        return []
    for previous, following in zip(periods, periods[1:], strict=False):
        if previous.served_until is None or following.served_from < previous.served_until:
            raise ImportValidationError(f"Member {member_id}: overlapping service periods")
    active = any(p.served_until is None or p.served_until > as_of for p in periods)
    if active != current:
        raise ImportValidationError(f"Member {member_id}: history disagrees with current search")
    return periods
