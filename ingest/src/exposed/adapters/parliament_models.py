"""Decode Parliament JSON at the HTTP edge; keep source aliases out of the core."""

from datetime import date, datetime
from typing import Annotated, Self

from pydantic import BeforeValidator, ConfigDict, Field, RootModel

from exposed.core.models import (
    DisplayName,
    HouseNumber,
    MemberProfile,
    Model,
    PositiveID,
)
from exposed.core.models import (
    HouseMembership as DomainHouseMembership,
)
from exposed.core.models import (
    MemberHistory as DomainMemberHistory,
)


def calendar_date(value: str) -> date:
    """Retain the source's calendar date, discarding time without timezone conversion."""
    try:
        return datetime.fromisoformat(value).date()
    except (ValueError, TypeError) as exc:
        raise ValueError("expected an ISO date or datetime") from exc


type SourceDate = Annotated[date, BeforeValidator(calendar_date)]


class HouseMembership(DomainHouseMembership):
    start_date: SourceDate = Field(validation_alias="membershipStartDate")
    end_date: SourceDate | None = Field(default=None, validation_alias="membershipEndDate")


class MemberHistory(DomainMemberHistory):
    parliament_member_id: PositiveID = Field(validation_alias="id")
    house_memberships: tuple[
        Annotated[DomainHouseMembership, BeforeValidator(HouseMembership.model_validate)], ...
    ] = Field(validation_alias="houseMembershipHistory", min_length=1)

    def to_history(self) -> DomainMemberHistory:
        return DomainMemberHistory(
            parliament_member_id=self.parliament_member_id,
            house_memberships=tuple(
                DomainHouseMembership(
                    house=membership.house,
                    start_date=membership.start_date,
                    end_date=membership.end_date,
                )
                for membership in self.house_memberships
            ),
        )


class PartyResponse(Model):
    id: PositiveID | None = None
    name: str | None = None


class LatestMembershipResponse(Model):
    house: HouseNumber
    membership_from: str | None = Field(default=None, validation_alias="membershipFrom")


class MemberResponse(Model):
    """Only the profile fields we consume from Parliament's nested response."""

    id: PositiveID
    name: DisplayName = Field(validation_alias="nameDisplayAs")
    party: PartyResponse | None = Field(default=None, validation_alias="latestParty")
    latest_membership: LatestMembershipResponse = Field(validation_alias="latestHouseMembership")

    def to_profile(self) -> MemberProfile:
        return MemberProfile(
            parliament_member_id=self.id,
            name=self.name,
            party_id=self.party.id if self.party else None,
            party_name=self.party.name if self.party else None,
            latest_house=self.latest_membership.house,
            latest_membership_from=self.latest_membership.membership_from,
        )


class ResponseItem[T](Model):
    """Parliament wraps each resource in a value field alongside unused metadata."""

    value: T


class SearchResponse(Model):
    items: tuple[ResponseItem[MemberResponse], ...]
    total_results: Annotated[int, Field(ge=0)] = Field(validation_alias="totalResults")
    skip: Annotated[int, Field(ge=0)]


class HistoryResponse(RootModel[tuple[ResponseItem[MemberHistory], ...]]):
    model_config = ConfigDict(strict=True, frozen=True)


class SearchPage(Model):
    """One page of profiles; collection checks belong to the importer."""

    members: tuple[MemberProfile, ...]
    total_results: Annotated[int, Field(ge=0)]
    skip: Annotated[int, Field(ge=0)]

    @classmethod
    def from_json(cls, payload: str | bytes) -> Self:
        response = SearchResponse.model_validate_json(payload)
        return cls(
            members=tuple(item.value.to_profile() for item in response.items),
            total_results=response.total_results,
            skip=response.skip,
        )


class HistoryBatch(Model):
    """Histories in response order, retaining duplicates for the importer to check."""

    histories: tuple[MemberHistory, ...]

    @classmethod
    def from_json(cls, payload: str | bytes) -> Self:
        response = HistoryResponse.model_validate_json(payload)
        return cls(histories=tuple(item.value for item in response.root))
