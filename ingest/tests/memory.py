"""In-memory implementations of the refresh ports used by boundary tests."""

from collections.abc import Iterator, Sequence
from contextlib import contextmanager
from datetime import date

from exposed.core.models import (
    HouseMembership,
    Member,
    MemberHistory,
    MemberProfile,
    ServicePeriod,
    validate_configured_term,
)
from exposed.core.ports import MemberWriter, WriteResult


class MemorySource:
    def __init__(self):
        self.profiles = (
            MemberProfile(
                parliament_member_id=1,
                name="Current member",
                latest_house=1,
                party_id=1,
                party_name="Example party",
                latest_membership_from="Example constituency",
            ),
            MemberProfile(
                parliament_member_id=2,
                name="Former member",
                latest_house=2,
                party_id=1,
                party_name="Example party",
                latest_membership_from="Example membership",
            ),
        )

    def current_commons(self) -> Iterator[MemberProfile]:
        yield self.profiles[0]

    def commons_candidates(
        self, term_start: date, as_of: date
    ) -> Iterator[tuple[MemberProfile, ...]]:
        yield self.profiles

    def member_histories(self, member_ids: set[int]) -> tuple[MemberHistory, ...]:
        return tuple(
            MemberHistory(
                parliament_member_id=member_id,
                house_memberships=(
                    HouseMembership(
                        house=1,
                        start_date=date(2019, 12, 12),
                        end_date=None if member_id == 1 else date(2025, 3, 17),
                    ),
                ),
            )
            for member_id in sorted(member_ids)
        )


class MemoryWriter:
    def __init__(self, members: dict[int, Member], service: dict[int, tuple[ServicePeriod, ...]]):
        self.members = members
        self.service = service

    def write_member(self, member: Member, periods: Sequence[ServicePeriod]) -> WriteResult:
        previous = self.members.get(member.parliament_member_id)
        self.members[member.parliament_member_id] = member
        self.service[member.parliament_member_id] = tuple(periods)
        if previous is None:
            return "inserted"
        return "unchanged" if previous == member else "updated"


class MemoryStore:
    def __init__(self):
        self.members: dict[int, Member] = {}
        self.service: dict[int, tuple[ServicePeriod, ...]] = {}
        self.term_starts: set[date] = set()

    @contextmanager
    def refresh_batch(self, term_start: date) -> Iterator[MemberWriter]:
        validate_configured_term(term_start, self.term_starts)
        pending = dict(self.members)
        pending_service = dict(self.service)
        yield MemoryWriter(pending, pending_service)
        self.members = pending
        self.service = pending_service
        self.term_starts.add(term_start)
