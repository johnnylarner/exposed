"""Parse Parliament JSON into typed models and validate individual fields."""

from datetime import date, datetime
from typing import Annotated, Self

from pydantic import BaseModel, BeforeValidator, ConfigDict, Field, RootModel, ValidationError


class ImportValidationError(ValueError):
    """The source is incomplete, inconsistent, or unsuitable for this import."""


def validation_error_message(error: ValidationError) -> str:
    """Report model and field paths without including raw input values."""
    details = []
    for issue in error.errors(include_url=False, include_context=False, include_input=False):
        path = ".".join(str(part) for part in issue["loc"]) or "response"
        details.append(f"{path}: {issue['msg']}")
    return f"Invalid {error.title}: {'; '.join(details)}"


def calendar_date(value: object) -> object:
    """Retain the source's calendar date, discarding time without timezone conversion."""
    if isinstance(value, str):
        try:
            return datetime.fromisoformat(value).date()
        except ValueError as exc:
            raise ValueError("expected an ISO date or datetime") from exc
    if isinstance(value, datetime):
        return value.date()
    return value


type PositiveID = Annotated[int, Field(gt=0)]
type HouseNumber = Annotated[int, Field(ge=1, le=2)]
type DisplayName = Annotated[str, Field(pattern=r"\S")]
type SourceDate = Annotated[date, BeforeValidator(calendar_date)]


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


class HouseMembership(Model):
    house: HouseNumber
    start_date: SourceDate = Field(validation_alias="membershipStartDate")
    end_date: SourceDate | None = Field(default=None, validation_alias="membershipEndDate")


class MemberHistory(Model):
    parliament_member_id: PositiveID = Field(validation_alias="id")
    house_memberships: tuple[HouseMembership, ...] = Field(
        validation_alias="houseMembershipHistory", min_length=1
    )


class ServicePeriod(Model):
    parliament_member_id: PositiveID
    source_start_date: date
    source_end_date: date | None = None
    served_from: date
    served_until: date | None = None
    house: Annotated[int, Field(ge=1, le=1)] = 1


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
