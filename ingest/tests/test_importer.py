from datetime import date

import httpx
import psycopg
import pytest
from psycopg import sql

import exposed.importer as importer
from exposed.db import DatabaseConnection
from exposed.importer import ImportFailed, connect, run_import
from tests.conftest import dbmate
from tests.fakes import AS_OF, TERM_START, ParliamentFixture, service

pytestmark = pytest.mark.integration


def run(url: str, fixture: ParliamentFixture, term_start: date = TERM_START) -> dict[str, object]:
    api = fixture.api()
    with api.client:
        return run_import(url, term_start, api, as_of=AS_OF)


def dataset(url: str):
    with connect(url) as conn:
        return {
            table: conn.execute(
                sql.SQL("SELECT * FROM exposed.{} ORDER BY id").format(sql.Identifier(table))
            ).fetchall()
            for table in ["members", "parliament_terms", "member_terms"]
        }


def test_repeat_import_keeps_ids_and_data(database_url):
    fixture = ParliamentFixture()
    fixture.leave(2, lords=True)
    first = run(database_url, fixture)
    before = dataset(database_url)
    second = run(database_url, fixture)
    after = dataset(database_url)
    assert (first["members"], first["current_commons"], first["former_commons"]) == (3, 2, 1)
    assert first["inserted"] == 3
    assert (second["inserted"], second["updated"], second["unchanged"]) == (0, 0, 3)
    for table in ["members", "parliament_terms", "member_terms"]:
        assert [r["id"] for r in before[table]] == [r["id"] for r in after[table]]
        assert all(r["id"].version == 7 for r in after[table])
    assert after == before
    assert "run_id" not in second
    assert "raw_responses" not in second


def test_profile_changes_departures_and_new_members_are_reconciled(database_url):
    fixture = ParliamentFixture(2)
    run(database_url, fixture)
    old_ids = {r["parliament_member_id"]: r["id"] for r in dataset(database_url)["members"]}
    fixture.leave(1)
    fixture.profiles[2]["latestParty"] = {"id": 8, "name": "Independent"}
    fixture.profiles[3] = {**fixture.profiles[2], "id": 3, "nameDisplayAs": "A replacement"}
    fixture.histories[3] = {"id": 3, "houseMembershipHistory": [service("2025-05-01")]}
    fixture.current.add(3)
    result = run(database_url, fixture)
    assert (result["inserted"], result["updated"], result["members"]) == (1, 2, 3)
    members = {r["parliament_member_id"]: r for r in dataset(database_url)["members"]}
    assert members[1]["id"] == old_ids[1]
    assert members[1]["is_current_commons"] is False
    assert members[2]["party_name"] == "Independent"
    periods = {r["member_id"]: r for r in dataset(database_url)["member_terms"]}
    assert periods[old_ids[1]]["served_until"] == date(2025, 3, 17)


def test_later_api_page_failure_rolls_back_batches_already_written(database_url, monkeypatch):
    fixture = ParliamentFixture(101)
    run(database_url, fixture)
    before = dataset(database_url)
    fixture.profiles[1]["nameDisplayAs"] = "Changed before a failed refresh"
    connections: list[DatabaseConnection] = []

    def track_connect(url: str) -> DatabaseConnection:
        conn = connect(url)
        connections.append(conn)
        return conn

    monkeypatch.setattr(importer, "connect", track_connect)
    observed: list[bool] = []

    def fail_second_page(request: httpx.Request) -> httpx.Response | None:
        if request.url.params.get("IsCurrentMember") or request.url.params.get("skip") != "100":
            return None
        # The first batch really has been written, but other sessions still see the old data.
        query = "SELECT name FROM exposed.members WHERE parliament_member_id = 1"
        assert next(connections[0].execute(query))["name"] == "Changed before a failed refresh"
        with connect(database_url) as reader:
            assert next(reader.execute(query))["name"] == "Example Member 1"
        observed.append(True)
        return httpx.Response(503)

    fixture.override = fail_second_page
    with pytest.raises(ImportFailed, match="503"):
        run(database_url, fixture)
    assert observed
    assert dataset(database_url) == before


def test_database_failure_after_member_writes_rolls_back_everything(database_url):
    fixture = ParliamentFixture(101)
    run(database_url, fixture)
    before = dataset(database_url)
    fixture.profiles[1]["nameDisplayAs"] = "Must roll back"
    fixture.leave(2)
    with connect(database_url) as conn:
        # Fail member 101 after the first 100 member and service writes.
        conn.execute(
            """ALTER TABLE exposed.members ADD CONSTRAINT fail_second_page
               CHECK (parliament_member_id <> 101) NOT VALID"""
        )
    with pytest.raises(ImportFailed, match="23514"):
        run(database_url, fixture)
    assert dataset(database_url) == before


def test_disappearing_historical_member_does_not_delete_or_deactivate_records(database_url):
    fixture = ParliamentFixture()
    run(database_url, fixture)
    before = dataset(database_url)
    fixture.profiles.pop(2)
    fixture.histories.pop(2)
    fixture.current.remove(2)
    result = run(database_url, fixture)
    assert result["status"] == "succeeded"
    assert result["members"] == 2
    assert dataset(database_url) == before


def test_future_term_start_is_rejected_before_api_fetch(database_url):
    fixture = ParliamentFixture()
    with pytest.raises(ImportFailed, match="after the import date"):
        run(database_url, fixture, date(2030, 1, 1))
    assert fixture.requests == []


def test_current_member_missing_from_historical_search_rolls_back(database_url):
    fixture = ParliamentFixture(2)

    def omit_member(request: httpx.Request) -> httpx.Response | None:
        if "MembershipInDateRange.WasMemberOfHouse" in request.url.params:
            return httpx.Response(
                200,
                json={
                    "items": [{"value": fixture.profiles[1]}],
                    "totalResults": 1,
                    "skip": 0,
                    "take": 100,
                },
            )
        return None

    fixture.override = omit_member
    with pytest.raises(ImportFailed, match="missing from historical"):
        run(database_url, fixture)
    assert all(not rows for rows in dataset(database_url).values())


def test_historical_candidate_without_service_in_term_is_excluded(database_url):
    fixture = ParliamentFixture(2)
    fixture.current.remove(2)
    fixture.histories[2]["houseMembershipHistory"] = [service("2019-12-12", "2024-05-30")]
    result = run(database_url, fixture)
    assert result["members"] == 1
    assert result["excluded_candidates"] == 1


def test_term_change_is_explicitly_rejected_for_this_first_version(database_url):
    fixture = ParliamentFixture()
    run(database_url, fixture)
    before = dataset(database_url)
    with pytest.raises(ImportFailed, match="another term"):
        run(database_url, fixture, date(2024, 7, 9))
    assert dataset(database_url) == before


def test_database_prevents_service_start_outside_term(database_url):
    run(database_url, ParliamentFixture())
    with connect(database_url) as conn, pytest.raises(psycopg.errors.CheckViolation):
        conn.execute("UPDATE exposed.member_terms SET served_from = '1987-06-11'")


def test_corrected_service_dates_replace_old_intervals(database_url):
    fixture = ParliamentFixture(1)
    fixture.histories[1]["houseMembershipHistory"] = [service("2025-05-02")]
    run(database_url, fixture)
    fixture.histories[1]["houseMembershipHistory"] = [service("2025-05-01")]
    run(database_url, fixture)
    data = dataset(database_url)
    assert len(data["member_terms"]) == 1
    assert data["member_terms"][0]["served_from"] == date(2025, 5, 1)


def test_remove_membership_from_id_preserves_member_data(database_url):
    run(database_url, ParliamentFixture())
    before = dataset(database_url)
    dbmate(database_url, "rollback")
    with connect(database_url) as conn:
        conn.execute("UPDATE exposed.members SET latest_membership_from_id = 101")
    dbmate(database_url)
    assert dataset(database_url) == before
    with connect(database_url) as conn:
        assert (
            conn.execute(
                """SELECT column_name FROM information_schema.columns
               WHERE table_schema = 'exposed' AND table_name = 'members'
                 AND column_name = 'latest_membership_from_id'"""
            ).fetchone()
            is None
        )
