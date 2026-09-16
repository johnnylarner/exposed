"""Import rules operate on typed pages and histories, independently of PostgreSQL."""

import httpx
import pytest

from exposed.importer import load_histories, search_pages
from exposed.models import ImportValidationError
from tests.fakes import ParliamentFixture

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


def test_empty_page_before_the_end_fails_instead_of_repeating_the_request():
    fixture = ParliamentFixture(101)

    def empty_second_page(request: httpx.Request) -> httpx.Response | None:
        if request.url.params.get("skip") == "100":
            return httpx.Response(200, json={"items": [], "totalResults": 101, "skip": 100})
        return None

    fixture.override = empty_second_page
    api = fixture.api()
    with api.client, pytest.raises(ImportValidationError, match="ended before"):
        list(search_pages(api, HISTORICAL_FILTERS))
    assert len(fixture.requests) == 2


def test_pagination_follows_the_latest_reported_total():
    fixture = ParliamentFixture(201)

    def smaller_initial_total(request: httpx.Request) -> httpx.Response | None:
        if request.url.params.get("skip") == "0":
            return httpx.Response(
                200,
                json={
                    "items": [{"value": fixture.profiles[i]} for i in range(1, 101)],
                    "totalResults": 150,
                    "skip": 0,
                },
            )
        return None

    fixture.override = smaller_initial_total
    api = fixture.api()
    with api.client:
        pages = list(search_pages(api, HISTORICAL_FILTERS))
    assert [len(page.members) for page in pages] == [100, 100, 1]


def test_missing_history_is_rejected():
    fixture = ParliamentFixture()
    fixture.override = lambda _: httpx.Response(200, json=[])
    api = fixture.api()
    with api.client, pytest.raises(ImportValidationError, match="all requested member IDs"):
        load_histories(api, set(fixture.profiles))


@pytest.mark.parametrize(
    ("returned_ids", "message"),
    [
        pytest.param([1, 1, 2], "Duplicate history", id="duplicate"),
        pytest.param([1, 3], "all requested member IDs", id="unexpected_id"),
        pytest.param([1], "all requested member IDs", id="missing_id"),
    ],
)
def test_history_batch_must_match_requested_members(returned_ids, message):
    fixture = ParliamentFixture(2)
    fixture.override = lambda _: httpx.Response(
        200,
        json=[{"value": {**fixture.histories[1], "id": member_id}} for member_id in returned_ids],
    )
    api = fixture.api()
    with api.client, pytest.raises(ImportValidationError, match=message):
        load_histories(api, {1, 2})


@pytest.mark.parametrize("count", [0, 101])
def test_invalid_history_request_size_fails_before_http(count):
    fixture = ParliamentFixture(count)
    api = fixture.api()
    with api.client, pytest.raises(ImportValidationError, match="between 1 and 100"):
        load_histories(api, set(fixture.profiles))
    assert fixture.requests == []
