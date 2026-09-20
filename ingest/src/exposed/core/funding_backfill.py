"""Match source funding to stored entries without changing financial data or identity."""

from collections import Counter, defaultdict
from collections.abc import Sequence
from decimal import Decimal

from exposed.core.declarations import FundingEntry
from exposed.core.errors import DeclarationParseError

type FundingKey = tuple[str | None, Decimal | None, str | None, str | None]


def funding_key(entry: FundingEntry) -> FundingKey:
    return entry.funder, entry.amount, entry.currency, entry.payment_type


def plan_funding_backfill(
    stored: Sequence[FundingEntry], source: Sequence[FundingEntry]
) -> tuple[FundingEntry, ...]:
    """Return enriched entries in stored order, or reject the entire declaration.

    Row order is not identity. Indistinguishable duplicates can be enriched only
    when their new metadata agrees. Existing non-null values are never overwritten.
    """
    if Counter(map(funding_key, stored)) != Counter(map(funding_key, source)):
        raise DeclarationParseError("Stored funding differs from the API; refresh or review first")
    metadata: dict[FundingKey, set[tuple[str | None, str | None]]] = defaultdict(set)
    for entry in source:
        metadata[funding_key(entry)].add((entry.donor_status, entry.company_number))
    enriched = []
    for entry in stored:
        candidates = metadata[funding_key(entry)]
        if len(candidates) != 1:
            raise DeclarationParseError("Duplicate funding entries have ambiguous donor metadata")
        status, number = next(iter(candidates))
        if (entry.donor_status is not None and entry.donor_status != status) or (
            entry.company_number is not None and entry.company_number != number
        ):
            raise DeclarationParseError("Existing donor metadata conflicts with the API")
        enriched.append(entry.model_copy(update={"donor_status": status, "company_number": number}))
    return tuple(enriched)
