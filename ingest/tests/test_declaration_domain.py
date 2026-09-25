from datetime import date
from decimal import Decimal

import pytest

from exposed.core.declarations import (
    DeclarationDraft,
    FundingEntry,
    PublishedVersion,
    latest_version,
)
from exposed.core.errors import DeclarationParseError


def test_parent_completion_uses_domain_values_without_parliament_json():
    parent = DeclarationDraft(
        id=500,
        member_source_id=1,
        category_id=12,
        category_name="Employment",
        funding=(),
        payer="Publisher",
    ).accept()
    child = DeclarationDraft(
        id=101,
        member_source_id=1,
        category_id=1,
        category_name="Payments",
        parent_id=500,
        funding=(
            FundingEntry(
                funder_name=None, amount=Decimal("340"), currency="GBP", payment_type="Monetary"
            ),
        ),
        payer=None,
    )
    assert child.accept(parent=parent).funding[0].funder_name == "Publisher"
    assert child.funding[0].funder_name is None
    with pytest.raises(DeclarationParseError, match="member mismatch"):
        child.model_copy(update={"member_source_id": 2}).accept(parent=parent)


def test_version_policy_uses_publication_dates_and_rejects_ambiguous_content():
    versions = (
        PublishedVersion(published_on=date(2024, 1, 1), content={"evidence": 2}),
        PublishedVersion(published_on=date(2023, 1, 1), content={"evidence": 1}),
    )
    assert latest_version(versions) == 0
    with pytest.raises(DeclarationParseError, match="conflicting versions"):
        latest_version(
            (versions[0], PublishedVersion(published_on=date(2024, 1, 1), content={"evidence": 3}))
        )


@pytest.mark.parametrize(
    "ultimate, donor, payer, grouped, expected",
    [
        ("Ultimate", "Donor", "Payer", "Grouped", "Ultimate"),
        (None, "Donor", "Payer", "Grouped", "Donor"),
        (None, None, "Payer", "Grouped", "Payer"),
        (None, None, None, "Grouped", "Grouped"),
        (None, None, None, None, None),
    ],
)
def test_funder_preference_is_a_domain_policy(ultimate, donor, payer, grouped, expected):
    from exposed.core.declarations import preferred_funder

    assert (
        preferred_funder(ultimate_payer=ultimate, donor=donor, payer=payer, group_donor=grouped)
        == expected
    )


def test_only_a_needed_name_decoding_failure_rejects_the_attribution():
    from exposed.core.declarations import preferred_funder

    invalid = DeclarationParseError("donor", {"bad": "name"}, "expected a string")
    assert preferred_funder(ultimate_payer="Ultimate", donor=invalid) == "Ultimate"
    with pytest.raises(DeclarationParseError, match="expected a string"):
        preferred_funder(donor=invalid, payer="Fallback")
