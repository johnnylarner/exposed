import pytest

from exposed.adapters.declaration_models import SourceDeclaration
from exposed.core.errors import DeclarationParseError
from tests.declaration_fakes import declaration, field, money


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
    assert result.funder_kind == (status or None)
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
    assert [(e.funder_name, e.funder_kind, e.company_number) for e in result.funding] == [
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
    assert result.funder_name == "Another funder"
    assert result.funder_kind is result.company_number is None


def test_latest_publication_supplies_metadata():
    from copy import deepcopy

    source = declaration(
        fields=[field("DonorName", "Donor"), money(), field("DonorStatus", "Company")]
    )
    older = deepcopy(source["versions"][0])
    older["register"]["publishedDate"] = "2023-01-01"
    older["fields"][-1]["value"] = "Individual"
    source["versions"].insert(0, older)
    assert SourceDeclaration.model_validate(source).to_draft().funding[0].funder_kind == "Company"
