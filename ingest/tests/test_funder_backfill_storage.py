import httpx
import pytest

from exposed.adapters.postgres import connect
from exposed.backfill_funders import backfill_funders
from exposed.core.errors import ImportValidationError, SourceError
from tests.declaration_fakes import DeclarationsFixture, declaration, field, money
from tests.fakes import ParliamentFixture
from tests.test_declaration_importer import dataset
from tests.test_declaration_importer import run as import_declarations
from tests.test_importer import run as import_members

pytestmark = pytest.mark.integration


def fixture_data():
    return DeclarationsFixture(
        declaration(
            fields=[
                field("DonorName", "Company"),
                money(),
                field("DonorStatus", "Company"),
                field("DonorCompanyIdentifier", "00123456"),
            ]
        ),
        declaration(
            102, fields=[field("DonorName", "Person"), money(), field("DonorStatus", "Individual")]
        ),
    )


def seed(url):
    import_members(url, ParliamentFixture(1))
    fixture = fixture_data()
    import_declarations(url, fixture)
    with connect(url) as conn:
        conn.execute(
            "UPDATE exposed.funding_entries SET donor_status = NULL, company_number = NULL"
        )
    return fixture


def run(url, fixture, **kwargs):
    api = fixture.api()
    with connect(url) as conn, api.client:
        return backfill_funders(conn, api, **kwargs)


def test_preview_apply_and_rerun_preserve_ids_finances_and_declaration_metadata(database_url):
    fixture = seed(database_url)
    before = dataset(database_url)
    preview = run(database_url, fixture)
    assert preview["mode"] == "dry-run"
    assert preview["changed_entries"] == 2
    assert dataset(database_url) == before
    result = run(database_url, fixture, apply=True)
    assert result["status"] == "succeeded"
    assert result["changed_entries"] == 2
    after = dataset(database_url)
    assert after["declarations"] == before["declarations"]
    for old, new in zip(before["funding"], after["funding"], strict=True):
        assert {k: v for k, v in new.items() if k not in {"donor_status", "company_number"}} == {
            k: v for k, v in old.items() if k not in {"donor_status", "company_number"}
        }
    assert [(r["donor_status"], r["company_number"]) for r in after["funding"]] == [
        ("Company", "00123456"),
        ("Individual", None),
    ]
    assert run(database_url, fixture, apply=True)["changed_entries"] == 0
    assert dataset(database_url) == after
    # Normal imports keep the enriched rows and their UUIDs too.
    import_declarations(database_url, fixture)
    assert dataset(database_url)["funding"] == after["funding"]


@pytest.mark.parametrize("problem", ["changed", "missing", "malformed", "wrong_member", "conflict"])
def test_bad_declaration_is_reported_without_partial_enrichment(database_url, caplog, problem):
    fixture = seed(database_url)
    if problem == "changed":
        fixture.items[0]["versions"][0]["fields"][1]["value"] = "1.00"
    elif problem == "missing":
        fixture.items.pop(0)
    elif problem == "malformed":
        fixture.items[0]["versions"][0]["fields"][-1]["value"] = 123
    elif problem == "wrong_member":
        fixture.items[0]["registrant"]["memberDetail"]["id"] = 2
    else:
        with connect(database_url) as conn:
            conn.execute(
                """UPDATE exposed.funding_entries SET donor_status = 'Individual'
                   WHERE source_declaration_id = 101"""
            )
    before = dataset(database_url)
    result = run(database_url, fixture, apply=True)
    assert result["status"] == "partial"
    assert result["skipped_declarations"] == 1
    assert result["changed_entries"] == 1
    after = dataset(database_url)
    assert after["funding"][0] == before["funding"][0]
    assert after["funding"][1]["donor_status"] == "Individual"
    assert after["declarations"] == before["declarations"]
    assert "Skipped declaration 101" in caplog.text


def test_later_http_failure_keeps_completed_declarations_and_rerun_finishes(database_url):
    fixture = seed(database_url)
    fixture.override = lambda r: (
        httpx.Response(503) if r.url.params.get("InterestIds") == "102" else None
    )
    with pytest.raises(SourceError):
        run(database_url, fixture, apply=True, batch_size=1)
    rows = dataset(database_url)["funding"]
    assert rows[0]["company_number"] == "00123456"
    assert rows[1]["donor_status"] is None
    fixture.override = None
    assert run(database_url, fixture, apply=True, batch_size=1)["changed_entries"] == 1


def test_explicit_ids_limit_scope_and_unknown_ids_fail_before_requests(database_url):
    fixture = seed(database_url)
    fixture.requests.clear()
    with pytest.raises(ImportValidationError, match="No stored funding"):
        run(database_url, fixture, declaration_ids=[999])
    assert fixture.requests == []
    assert run(database_url, fixture, apply=True, declaration_ids=[102])["declarations"] == 1
    assert dataset(database_url)["funding"][0]["donor_status"] is None
    assert all(r.url.params.get_list("InterestIds") == ["102"] for r in fixture.requests)


def test_conflicting_duplicate_responses_cause_no_writes_in_batch(database_url):
    from copy import deepcopy

    fixture = seed(database_url)
    duplicate = deepcopy(fixture.items[0])
    duplicate["versions"][0]["fields"][-1]["value"] = "00999999"
    fixture.items.append(duplicate)
    before = dataset(database_url)
    with pytest.raises(ImportValidationError, match="Conflicting duplicate"):
        run(database_url, fixture, apply=True)
    assert dataset(database_url) == before
