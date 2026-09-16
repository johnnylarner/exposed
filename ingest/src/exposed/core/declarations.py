"""Declaration values and interpretation policies, independent of source formats."""

import json
from collections import Counter
from collections.abc import Sequence
from datetime import date, datetime
from decimal import Decimal

from pydantic import JsonValue

from exposed.core.errors import DeclarationParseError, ParentRequired
from exposed.core.models import DisplayName, Model, PositiveID


def canonical_json(value: JsonValue) -> str:
    """Compare evidence without conflating JSON booleans and numbers."""
    return json.dumps(value, sort_keys=True, separators=(",", ":"))


class PublishedVersion(Model):
    published_on: date
    content: JsonValue


def latest_version(versions: Sequence[PublishedVersion]) -> int:
    """Choose the newest publication; equally recent conflicting evidence is invalid."""
    if not versions:
        raise DeclarationParseError("versions", [], "expected at least one published version")
    newest = max(version.published_on for version in versions)
    latest = [i for i, version in enumerate(versions) if version.published_on == newest]
    first = latest[0]
    if any(
        canonical_json(versions[i].content) != canonical_json(versions[first].content)
        for i in latest[1:]
    ):
        raise DeclarationParseError(
            "versions",
            [v.content for v in versions],
            "conflicting versions at latest register date",
        )
    return first


type FunderName = str | None | DeclarationParseError


def preferred_funder(
    *,
    ultimate_payer: FunderName = None,
    donor: FunderName = None,
    payer: FunderName = None,
    group_donor: FunderName = None,
) -> str | None:
    """Prefer the ultimate payer, then direct donor/payer, then a grouped donor.

    A name-decoding failure matters only if that candidate is needed. This retains
    the existing treatment of malformed, unused fallback names without moving
    source field decoding into the domain.
    """
    for name in (ultimate_payer, donor, payer, group_donor):
        if isinstance(name, DeclarationParseError):
            raise name
        if name:
            return name
    return None


class FundingEntry(Model):
    funder: str | None
    amount: Decimal | None
    currency: str | None
    payment_type: str | None


class DeclarationIdentity(Model):
    id: PositiveID
    member_source_id: PositiveID
    category_id: PositiveID
    category_name: DisplayName


class Declaration(DeclarationIdentity):
    """A fully interpreted declaration, ready for atomic publication."""

    funding: tuple[FundingEntry, ...]
    payer: str | None

    def has_same_funding(self, previous: Sequence[FundingEntry]) -> bool:
        """Funding identity ignores order, while equal entries retain their multiplicity."""
        return Counter(self.funding) == Counter(previous)


class DeclarationDraft(DeclarationIdentity):
    """Complete local funding interpretation that may still require a parent payer."""

    funding: tuple[FundingEntry, ...]
    payer: str | None
    parent_id: PositiveID | None = None
    ultimate_payer_differs: bool = False

    def accept(self, *, parent: Declaration | None = None) -> Declaration:
        funding, payer = self.funding, self.payer
        needs_parent = (
            self.parent_id is not None
            and not self.ultimate_payer_differs
            and (any(entry.funder is None for entry in funding) or (not funding and payer is None))
        )
        if needs_parent and self.parent_id is not None:
            if parent is None:
                raise ParentRequired(self.parent_id)
            if parent.id != self.parent_id or parent.member_source_id != self.member_source_id:
                raise DeclarationParseError(
                    "parent_id", self.parent_id, "parent identity or member mismatch"
                )
            if parent.payer is None:
                raise DeclarationParseError(
                    "parent_id", self.parent_id, "required parent payer is absent"
                )
            payer = payer or parent.payer
            funding = tuple(
                FundingEntry(
                    funder=entry.funder or parent.payer,
                    amount=entry.amount,
                    currency=entry.currency,
                    payment_type=entry.payment_type,
                )
                for entry in funding
            )
        return Declaration(
            id=self.id,
            member_source_id=self.member_source_id,
            category_id=self.category_id,
            category_name=self.category_name,
            funding=funding,
            payer=payer,
        )


class RetrievedDeclaration(Model):
    """Transient source response and retrieval context; never persisted."""

    identifier: JsonValue
    payload: JsonValue
    fetched_at: datetime
    context: str

    @property
    def source_id(self) -> int | None:
        if isinstance(self.identifier, int) and not isinstance(self.identifier, bool):
            return self.identifier
        return None
