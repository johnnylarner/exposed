import httpx
import pytest

from exposed.api import APIError
from exposed.models import ImportValidationError
from tests.fakes import ParliamentFixture

HISTORICAL_FILTERS: dict[str, str | int] = {
    "MembershipInDateRange.WasMemberOnOrAfter": "2024-07-04",
    "MembershipInDateRange.WasMemberOnOrBefore": "2026-09-15",
    "MembershipInDateRange.WasMemberOfHouse": 1,
}


def test_search_pages_include_departed_mp_now_in_lords():
    fixture = ParliamentFixture()
    fixture.leave(2, lords=True)
    api = fixture.api()
    with api.client:
        pages = list(api.search_pages(HISTORICAL_FILTERS))
    assert len(pages) == 1
    members = pages[0]
    assert set(members) == {1, 2, 3}
    assert members[2]["latestHouseMembership"]["house"] == 2


def test_pages_are_yielded_before_later_pages_are_requested():
    fixture = ParliamentFixture(41)
    api = fixture.api()
    with api.client:
        pages = api.search_pages(HISTORICAL_FILTERS)
        first = next(pages)
        assert set(first) == set(range(1, 21))
        assert len(fixture.requests) == 1
        rest = list(pages)
    assert [len(page) for page in rest] == [20, 1]
    assert [r.url.params["skip"] for r in fixture.requests] == ["0", "20", "40"]


@pytest.mark.parametrize("problem", ["duplicate", "changed_total", "empty_page", "extra_item"])
def test_incomplete_or_unstable_pagination_is_rejected(problem):
    fixture = ParliamentFixture(21)

    def override(request: httpx.Request) -> httpx.Response | None:
        if request.url.params.get("skip") != "20":
            return None
        member_id = 1 if problem == "duplicate" else 21
        items = [] if problem == "empty_page" else [{"value": fixture.profiles[member_id]}]
        if problem == "extra_item":
            items.append({"value": {**fixture.profiles[21], "id": 22}})
        return httpx.Response(
            200,
            json={
                "totalResults": 22 if problem == "changed_total" else 21,
                "skip": 20,
                "take": 20,
                "items": items,
            },
        )

    fixture.override = override
    api = fixture.api()
    with api.client, pytest.raises(ImportValidationError):
        list(api.search_pages(HISTORICAL_FILTERS))


def test_history_request_includes_every_id_in_batch():
    fixture = ParliamentFixture(20)
    api = fixture.api()
    with api.client:
        histories = api.histories(set(fixture.profiles))
    assert set(histories) == set(fixture.profiles)
    assert set(fixture.requests[0].url.params.get_list("ids")) == {str(i) for i in fixture.profiles}


def test_missing_history_is_rejected():
    fixture = ParliamentFixture()
    fixture.override = lambda r: httpx.Response(200, json=[])
    api = fixture.api()
    with api.client, pytest.raises(ImportValidationError, match="all requested member IDs"):
        api.histories(set(fixture.profiles))


def test_rate_limit_retries_honor_retry_after():
    fixture = ParliamentFixture(1)
    fixture.override = lambda r: (
        httpx.Response(429, headers={"Retry-After": "3"}) if len(fixture.requests) == 1 else None
    )
    api = fixture.api()
    with api.client:
        assert len(list(api.search_pages(HISTORICAL_FILTERS))) == 1
    assert 3 in fixture.sleeps


def test_persistent_server_failure_stops_after_four_attempts():
    fixture = ParliamentFixture(1)
    fixture.override = lambda r: httpx.Response(503)
    api = fixture.api()
    with api.client, pytest.raises(APIError, match="503"):
        list(api.search_pages(HISTORICAL_FILTERS))
    assert len(fixture.requests) == 4


def test_http_400_is_not_retried():
    fixture = ParliamentFixture(1)
    fixture.override = lambda r: httpx.Response(400)
    api = fixture.api()
    with api.client, pytest.raises(APIError, match="400"):
        list(api.search_pages(HISTORICAL_FILTERS))
    assert len(fixture.requests) == 1
