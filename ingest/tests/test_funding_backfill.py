from decimal import Decimal

import pytest

from exposed.adapters.declaration_models import SourceDeclaration
from exposed.core.declarations import FundingEntry
from exposed.core.errors import DeclarationParseError
from exposed.core.funding_backfill import plan_funding_backfill
from tests.declaration_fakes import declaration, field, money


def entry(name="Example Ltd", status=None, number=None):
    return FundingEntry(
        funder=name,
        amount=Decimal("2000.00"),
        currency="GBP",
        payment_type=None,
        donor_status=status,
        company_number=number,
    )


def parse(fields):
    return SourceDeclaration.model_validate(declaration(fields=fields)).to_draft().accept()


@pytest.mark.parametrize(
    "status,number,expected",
    [
        ("Company", "00123456", "00123456"),
        ("Company", "OC012345", "OC012345"),
        ("Company", None, None),
        ("Company", "  ", None),
        ("Individual", "00123456", None),
        ("Trade Union", "00123456", None),
        ("Future status", None, None),
        (None, "00123456", None),
        ("", None, None),
    ],
)
def test_explicit_donor_fields_preserve_source_status_and_company_identifiers(
    status, number, expected
):
    result = parse(
        [
            field("DonorName", "Donor"),
            money(),
            field("DonorStatus", status),
            field("DonorCompanyIdentifier", number),
        ]
    ).funding[0]
    assert result.donor_status == (status or None)
    assert result.company_number == expected


@pytest.mark.parametrize("name", ["DonorStatus", "DonorCompanyIdentifier"])
def test_invalid_metadata_rejects_declaration_instead_of_coercing_numbers(name):
    fields = [field("DonorName", "Donor"), money(), field("DonorStatus", "Company")]
    if name == "DonorStatus":
        fields[-1]["value"] = 123
    else:
        fields.append(field(name, 123))
    with pytest.raises(DeclarationParseError, match="expected a string"):
        parse(fields)


def test_donor_groups_keep_metadata_paired_without_inheriting_outer_fields():
    result = parse(
        [
            field("DonorStatus", "Company"),
            field("DonorCompanyIdentifier", "99999999"),
            field(
                "Donors",
                None,
                "Donor[]",
                values=[
                    [field("Name", "Person"), field("DonorStatus", "Individual"), money()],
                    [
                        field("Name", "Company"),
                        field("DonorStatus", "Company"),
                        field("DonorCompanyIdentifier", "00123456"),
                        money(),
                    ],
                    [
                        field("Name", "Unknown"),
                        field("IsPrivateIndividual", False, "Boolean"),
                        money(),
                    ],
                ],
            ),
        ]
    )
    assert [(e.funder, e.donor_status, e.company_number) for e in result.funding] == [
        ("Person", "Individual", None),
        ("Company", "Company", "00123456"),
        ("Unknown", None, None),
    ]


def test_donor_metadata_is_not_attached_to_a_different_ultimate_payer():
    result = parse(
        [
            field("UltimatePayerName", "Another funder"),
            field("DonorName", "Intermediary"),
            field("DonorStatus", "Company"),
            field("DonorCompanyIdentifier", "00123456"),
            money(),
        ]
    ).funding[0]
    assert result.funder == "Another funder"
    assert result.donor_status is result.company_number is None


def test_latest_publication_supplies_metadata():
    from copy import deepcopy

    source = declaration(
        fields=[field("DonorName", "Donor"), money(), field("DonorStatus", "Company")]
    )
    older = deepcopy(source["versions"][0])
    older["register"]["publishedDate"] = "2023-01-01"
    older["fields"][-1]["value"] = "Individual"
    source["versions"].insert(0, older)
    assert SourceDeclaration.model_validate(source).to_draft().funding[0].donor_status == "Company"


def test_backfill_matches_by_values_not_order_and_preserves_duplicate_multiplicity():
    stored = [entry("Person"), entry(), entry()]
    source = [
        entry(status="Company", number="00123456"),
        entry("Person", "Individual"),
        entry(status="Company", number="00123456"),
    ]
    planned = plan_funding_backfill(stored, source)
    assert planned == (source[1], source[0], source[2])
    assert plan_funding_backfill(planned, source) == planned


@pytest.mark.parametrize(
    "source",
    [
        [],
        [entry(), entry()],
        [entry("Different donor")],
        [entry().model_copy(update={"amount": Decimal("1")})],
    ],
)
def test_changed_funding_rejects_whole_declaration(source):
    with pytest.raises(DeclarationParseError, match="Stored funding differs"):
        plan_funding_backfill([entry()], source)


def test_ambiguous_duplicates_are_not_assigned_arbitrary_company_numbers():
    with pytest.raises(DeclarationParseError, match="ambiguous"):
        plan_funding_backfill(
            [entry(), entry()],
            [
                entry(status="Company", number="00123456"),
                entry(status="Company", number="00999999"),
            ],
        )


@pytest.mark.parametrize(
    "source", [entry(), entry(status="Individual"), entry(status="Company", number="00999999")]
)
def test_existing_metadata_is_not_cleared_or_overwritten(source):
    with pytest.raises(DeclarationParseError, match="conflicts"):
        plan_funding_backfill([entry(status="Company", number="00123456")], [source])
