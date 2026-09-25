from decimal import Decimal

import pytest

from exposed.importer import ImportFailed, connect
from tests.declaration_fakes import DeclarationsFixture, declaration, field, money
from tests.fakes import ParliamentFixture
from tests.test_declaration_importer import dataset, run
from tests.test_importer import run as import_members

pytestmark = pytest.mark.integration


def funders(url):
    with connect(url) as conn:
        return conn.execute("SELECT * FROM exposed.funders ORDER BY funder_name").fetchall()


def test_standardized_names_share_funders_across_members_without_collapsing_payments(
    database_url,
):
    import_members(database_url, ParliamentFixture(2))
    donor = [field("Name", " Shared Donor Limited "), money("12.50")]
    fixture = DeclarationsFixture(
        declaration(fields=[field("Donors", None, "Donor[]", values=[donor, donor])]),
        declaration(102, member=2, fields=[field("DonorName", "Shared Donor ltd"), money()]),
        declaration(103, member=2, fields=[field("DonorName", "SHARED DONOR LTD."), money()]),
    )
    run(database_url, fixture)
    before = dataset(database_url)
    identities = funders(database_url)
    assert len(identities) == 1
    assert all(f["id"].version == 7 for f in identities)
    shared = [r for r in before["funding"] if r["funder_name"] == "shared donor ltd"]
    assert len(shared) == 4
    assert len({r["id"] for r in shared}) == 4
    assert len({r["funder_id"] for r in shared}) == 1
    run(database_url, fixture)
    assert dataset(database_url)["funding"] == before["funding"]
    assert funders(database_url) == identities


def test_shared_metadata_updates_keep_funding_ids_and_missing_fields_keep_known_details(
    database_url,
):
    import_members(database_url, ParliamentFixture(1))
    source = declaration(fields=[field("DonorName", "Donor"), money()])
    fixture = DeclarationsFixture(source)
    run(database_url, fixture)
    before = dataset(database_url)["funding"]
    original_funder = funders(database_url)[0]
    fields = source["versions"][0]["fields"]
    fields.extend([field("DonorStatus", "Company"), field("DonorCompanyIdentifier", "SC001234")])
    run(database_url, fixture)
    enriched = funders(database_url)[0]
    assert enriched["id"] == original_funder["id"]
    assert enriched["created_at"] == original_funder["created_at"]
    assert enriched["updated_at"] > original_funder["updated_at"]
    assert (enriched["funder_kind"], enriched["company_number"]) == ("Company", "SC001234")
    fields[-1]["value"] = "00123456"
    run(database_url, fixture)
    assert funders(database_url)[0]["company_number"] == "00123456"
    fields[-1]["value"] = None
    fields[-2]["value"] = None
    run(database_url, fixture)
    assert funders(database_url)[0]["company_number"] == "00123456"
    assert funders(database_url)[0]["funder_kind"] == "Company"
    fields[-2]["value"] = "Individual"
    run(database_url, fixture)
    assert funders(database_url)[0]["company_number"] is None
    assert funders(database_url)[0]["funder_kind"] == "Individual"
    after = dataset(database_url)["funding"]
    assert [(r["id"], r["funder_id"], r["updated_at"]) for r in after] == [
        (r["id"], r["funder_id"], r["updated_at"]) for r in before
    ]


def test_unnamed_payments_remain_nullable_without_creating_placeholder_funders(database_url):
    import_members(database_url, ParliamentFixture(1))
    source = declaration(fields=[money("10", None)])
    fixture = DeclarationsFixture(source)
    run(database_url, fixture)
    before = dataset(database_url)["funding"]
    assert len(before) == 1
    assert before[0]["amount"] == Decimal("10")
    assert before[0]["funder_id"] is before[0]["currency"] is before[0]["payment_type"] is None
    assert funders(database_url) == []
    run(database_url, fixture)
    assert dataset(database_url)["funding"] == before
    source["versions"][0]["fields"].append(field("DonorName", "Now identified"))
    run(database_url, fixture)
    after = dataset(database_url)["funding"][0]
    assert after["id"] != before[0]["id"]
    assert after["funder_id"] == funders(database_url)[0]["id"]


def test_database_failure_rolls_back_new_funders_and_shared_metadata(database_url):
    import_members(database_url, ParliamentFixture(1))
    original = declaration(fields=[field("DonorName", "Existing"), money()])
    run(database_url, DeclarationsFixture(original))
    before = dataset(database_url)
    identities = funders(database_url)
    original["versions"][0]["fields"].append(field("DonorStatus", "Individual"))
    fixture = DeclarationsFixture(
        original,
        declaration(102, fields=[field("DonorName", "Must roll back"), money("999")]),
    )
    with connect(database_url) as conn:
        conn.execute("ALTER TABLE exposed.funding_entries ADD CHECK (amount <> 999)")
    with pytest.raises(ImportFailed, match="CheckViolation"):
        run(database_url, fixture)
    assert dataset(database_url) == before
    assert funders(database_url) == identities


def test_funder_foreign_key_and_company_number_constraint(database_url):
    import psycopg

    import_members(database_url, ParliamentFixture(1))
    run(
        database_url,
        DeclarationsFixture(declaration(fields=[field("DonorName", "Donor"), money()])),
    )
    with connect(database_url) as conn:
        with pytest.raises(psycopg.errors.ForeignKeyViolation):
            conn.execute("UPDATE exposed.funding_entries SET funder_id = uuidv7()")
        with pytest.raises(psycopg.errors.ForeignKeyViolation):
            conn.execute("DELETE FROM exposed.funders")
        with pytest.raises(psycopg.errors.CheckViolation):
            conn.execute("UPDATE exposed.funders SET company_number = '00123456'")
