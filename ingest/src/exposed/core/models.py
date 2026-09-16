"""Validated member values and Commons service rules, independent of source formats."""

from datetime import date
from itertools import pairwise
from typing import Annotated, Self

from pydantic import BaseModel, ConfigDict, Field, field_validator, model_validator

from exposed.core.errors import ImportValidationError

type PositiveID = Annotated[int, Field(gt=0)]
type HouseNumber = Annotated[int, Field(ge=1, le=2)]
type DisplayName = Annotated[str, Field(pattern=r"\S")]


class Model(BaseModel):
    model_config = ConfigDict(strict=True, extra="ignore", frozen=True, validate_by_name=True)


class MemberProfile(Model):
    """Latest profile, independent of current Commons membership."""

    parliament_member_id: PositiveID
    name: DisplayName
    party_id: PositiveID | None = None
    party_name: str | None = None
    latest_house: HouseNumber
    latest_membership_from: str | None = None


class Member(MemberProfile):
    """Member fields persisted by the importer."""

    is_current_commons: bool

    @classmethod
    def from_profile(cls, profile: MemberProfile, *, is_current_commons: bool) -> Self:
        return cls(**profile.model_dump(), is_current_commons=is_current_commons)

    @model_validator(mode="after")
    def current_membership_is_commons(self) -> Self:
        if self.is_current_commons and self.latest_house != 1:
            raise ValueError(f"Member {self.parliament_member_id}: inconsistent latest House")
        return self


class HouseMembership(Model):
    house: HouseNumber
    start_date: date
    end_date: date | None = None


class MemberHistory(Model):
    parliament_member_id: PositiveID
    house_memberships: tuple[HouseMembership, ...] = Field(min_length=1)


class ServicePeriod(Model):
    parliament_member_id: PositiveID
    source_start_date: date
    source_end_date: date | None = None
    served_from: date
    served_until: date | None = None
    house: Annotated[int, Field(ge=1, le=1)] = 1

    @classmethod
    def from_membership(
        cls, member_id: int, membership: HouseMembership, *, term_start: date
    ) -> Self:
        return cls(
            parliament_member_id=member_id,
            source_start_date=membership.start_date,
            source_end_date=membership.end_date,
            served_from=max(membership.start_date, term_start),
            served_until=membership.end_date,
        )

    @model_validator(mode="after")
    def dates_describe_valid_service(self) -> Self:
        if self.served_from < self.source_start_date:
            raise ValueError("Service cannot begin before the source start date")
        if self.served_until != self.source_end_date:
            raise ValueError("Service end must match the source end date")
        if self.served_until is not None and self.served_until < self.served_from:
            raise ValueError(f"Member {self.parliament_member_id}: service ends before it starts")
        return self


class CommonsService(Model):
    """One member's Commons service during a term, observed on a fixed date."""

    parliament_member_id: PositiveID
    as_of: date
    is_current_commons: bool
    periods: tuple[ServicePeriod, ...]

    @classmethod
    def from_history(
        cls,
        history: MemberHistory,
        *,
        term_start: date,
        as_of: date,
        is_current_commons: bool,
    ) -> Self:
        # An end date is the date service ceased, so service ending on term_start is excluded.
        memberships_in_term = [
            membership
            for membership in history.house_memberships
            if membership.house == 1
            and membership.start_date <= as_of
            and (membership.end_date is None or membership.end_date > term_start)
        ]
        return cls(
            parliament_member_id=history.parliament_member_id,
            as_of=as_of,
            is_current_commons=is_current_commons,
            periods=tuple(
                ServicePeriod.from_membership(
                    history.parliament_member_id, membership, term_start=term_start
                )
                for membership in memberships_in_term
            ),
        )

    @field_validator("periods")
    @classmethod
    def sort_distinct_periods(cls, periods: tuple[ServicePeriod, ...]) -> tuple[ServicePeriod, ...]:
        return tuple(sorted(set(periods), key=lambda period: period.source_start_date))

    @model_validator(mode="after")
    def periods_do_not_overlap(self) -> Self:
        for previous, following in pairwise(self.periods):
            if previous.source_start_date == following.source_start_date:
                raise ValueError(f"Member {self.parliament_member_id}: conflicting service periods")
            if previous.served_until is None or following.served_from < previous.served_until:
                raise ValueError(f"Member {self.parliament_member_id}: overlapping service periods")
        return self

    @model_validator(mode="after")
    def service_matches_current_membership(self) -> Self:
        if self.is_current_commons and not self.periods:
            raise ValueError(
                f"Current member {self.parliament_member_id} has no service in this term"
            )
        active = any(
            period.served_until is None or period.served_until > self.as_of
            for period in self.periods
        )
        if active != self.is_current_commons:
            raise ValueError(
                f"Member {self.parliament_member_id}: history disagrees with current search"
            )
        return self


def validate_configured_term(term_start: date, stored_starts: set[date]) -> None:
    """This importer refreshes one Parliament; stored terms cannot silently roll over."""
    if stored_starts - {term_start}:
        raise ImportValidationError(
            "This database contains another term. This first version refreshes one configured "
            "Parliament; term rollover requires an explicit migration."
        )
