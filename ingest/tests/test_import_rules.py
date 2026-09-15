"""Import rules operate on typed pages and histories, independently of PostgreSQL."""

import json
from datetime import date

import httpx
import pytest

from exposed.importer import build_member, load_histories, search_pages, service_periods
from exposed.models import ImportValidationError, MemberHistory, MemberResponse
from tests.fakes import AS_OF, TERM_START, ParliamentFixture, service

HISTORICAL_FILTERS: dict[str, str | int] = {
    "MembershipInDateRange.WasMemberOnOrAfter": "2024-07-04",
    "MembershipInDateRange.WasMemberOnOrBefore": "2026-09-15",
    "MembershipInDateRange.WasMemberOfHouse": 1,
}


def test_pages_are_yielded_before_later_pages_are_requested():
    fixture = ParliamentFixture(201)
    api = fixture.api()
    with api.client:
        pages = search_pages(api, HISTORICAL_FILTERS)
        first = next(pages)
        assert {member.parliament_member_id for member in first.members} == set(range(1, 101))
        assert len(fixture.requests) == 1
        rest = list(pages)
    assert [len(page.members) for page in rest] == [100, 1]
    assert [r.url.params["skip"] for r in fixture.requests] == ["0", "100", "200"]


@pytest.mark.parametrize("problem", ["duplicate", "changed_total", "empty_page", "extra_item"])
def test_incomplete_or_unstable_pagination_is_rejected(problem):
    fixture = ParliamentFixture(101)

    def override(request: httpx.Request) -> httpx.Response | None:
        if request.url.params.get("skip") != "100":
            return None
        member_id = 1 if problem == "duplicate" else 101
        items = [] if problem == "empty_page" else [{"value": fixture.profiles[member_id]}]
        if problem == "extra_item":
            items.append({"value": {**fixture.profiles[101], "id": 102}})
        return httpx.Response(
            200,
            json={
                "totalResults": 102 if problem == "changed_total" else 101,
                "skip": 100,
                "take": 100,
                "items": items,
            },
        )

    fixture.override = override
    api = fixture.api()
    with api.client, pytest.raises(ImportValidationError):
        list(search_pages(api, HISTORICAL_FILTERS))


def test_missing_history_is_rejected():
    fixture = ParliamentFixture()
    fixture.override = lambda r: httpx.Response(200, json=[])
    api = fixture.api()
    with api.client, pytest.raises(ImportValidationError, match="all requested member IDs"):
        load_histories(api, set(fixture.profiles))


def test_latest_profile_can_be_lords_while_commons_service_is_retained():
    fixture = ParliamentFixture(1)
    fixture.leave(1, lords=True)
    member = build_member(
        MemberResponse.model_validate(fixture.profiles[1]).to_profile(), current=False
    )
    periods = service_periods(
        MemberHistory.model_validate_json(json.dumps(fixture.histories[1])),
        TERM_START,
        AS_OF,
        current=False,
    )
    assert member.latest_house == 2
    assert not member.is_current_commons
    assert len(periods) == 1
    assert periods[0].served_until == date(2025, 3, 17)


def test_long_continuous_source_period_is_clipped_without_losing_original_date():
    history = {"houseMembershipHistory": [service("1987-06-11")]}
    period = service_periods(
        MemberHistory.model_validate_json(json.dumps({"id": 1, **history})),
        TERM_START,
        AS_OF,
        current=True,
    )[0]
    assert period.served_from == TERM_START
    assert period.source_start_date == date(1987, 6, 11)


def test_by_election_service_starts_on_its_source_date():
    history = {"houseMembershipHistory": [service("2025-05-01")]}
    period = service_periods(
        MemberHistory.model_validate_json(json.dumps({"id": 1, **history})),
        TERM_START,
        AS_OF,
        current=True,
    )[0]
    assert period.served_from == date(2025, 5, 1)


def test_reentry_retains_a_real_service_gap():
    history = {"houseMembershipHistory": [service(end="2025-01-01"), service("2025-06-01")]}
    periods = service_periods(
        MemberHistory.model_validate_json(json.dumps({"id": 1, **history})),
        TERM_START,
        AS_OF,
        current=True,
    )
    assert len(periods) == 2
    assert periods[0].served_until == date(2025, 1, 1)
    assert periods[1].served_from == date(2025, 6, 1)


def test_disagreement_between_history_and_current_cohort_is_rejected():
    history = {"houseMembershipHistory": [service(end="2025-01-01")]}
    with pytest.raises(ImportValidationError, match="disagrees"):
        service_periods(
            MemberHistory.model_validate_json(json.dumps({"id": 1, **history})),
            TERM_START,
            AS_OF,
            current=True,
        )


@pytest.mark.parametrize("end", ["2024-05-30", "2024-07-04"])
def test_service_that_ceased_before_or_on_term_start_is_excluded(end):
    history = {"houseMembershipHistory": [service("2019-12-12", end)]}
    assert (
        service_periods(
            MemberHistory.model_validate_json(json.dumps({"id": 1, **history})),
            TERM_START,
            AS_OF,
            current=False,
        )
        == []
    )


@pytest.mark.parametrize("problem", ["duplicate", "unexpected_id", "missing_id"])
def test_history_batch_must_match_requested_members(problem):
    fixture = ParliamentFixture(2)
    returned_ids = {"duplicate": [1, 1, 2], "unexpected_id": [1, 3], "missing_id": [1]}[problem]
    fixture.override = lambda r: httpx.Response(
        200,
        json=[{"value": {**fixture.histories[1], "id": member_id}} for member_id in returned_ids],
    )
    api = fixture.api()
    with api.client, pytest.raises(ImportValidationError):
        load_histories(api, {1, 2})


@pytest.mark.parametrize("count", [0, 101])
def test_invalid_history_request_size_fails_before_http(count):
    fixture = ParliamentFixture(count)
    api = fixture.api()
    with api.client, pytest.raises(ImportValidationError, match="between 1 and 100"):
        load_histories(api, set(fixture.profiles))
    assert fixture.requests == []


@pytest.mark.parametrize("problem", ["duplicate_in_page", "oversized_page", "wrong_offset"])
def test_page_batch_rules_are_enforced_by_importer(problem):
    fixture = ParliamentFixture(101)
    ids = [1, 1] if problem == "duplicate_in_page" else list(fixture.profiles)
    if problem == "wrong_offset":
        ids = [1]
    fixture.override = lambda r: httpx.Response(
        200,
        json={
            "items": [{"value": fixture.profiles[member_id]} for member_id in ids],
            "totalResults": len(ids),
            "skip": 1 if problem == "wrong_offset" else 0,
        },
    )
    api = fixture.api()
    with api.client, pytest.raises(ImportValidationError):
        list(search_pages(api, HISTORICAL_FILTERS))


@pytest.mark.parametrize(
    ("memberships", "message"),
    [
        ([service("2025-06-01", "2025-05-01")], "ends before"),
        ([service(), service(end="2025-05-01")], "conflicting"),
        ([service(), service("2025-05-01")], "overlapping"),
        ([service("2024-07-04", "2025-06-01"), service("2025-05-01")], "overlapping"),
    ],
)
def test_service_period_relationships_are_checked_in_importer(memberships, message):
    history = MemberHistory.model_validate_json(
        json.dumps({"id": 1, "houseMembershipHistory": memberships})
    )
    with pytest.raises(ImportValidationError, match=message):
        service_periods(history, TERM_START, AS_OF, current=True)


def test_identical_source_periods_are_deduplicated():
    history = MemberHistory.model_validate_json(
        json.dumps({"id": 1, "houseMembershipHistory": [service(), service()]})
    )
    assert len(service_periods(history, TERM_START, AS_OF, current=True)) == 1


def test_current_member_requires_commons_profile():
    fixture = ParliamentFixture(1)
    fixture.leave(1, lords=True)
    profile = MemberResponse.model_validate(fixture.profiles[1]).to_profile()
    with pytest.raises(ImportValidationError, match="inconsistent latest House"):
        build_member(profile, current=True)
