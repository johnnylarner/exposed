from datetime import date

import httpx
import pytest

from exposed.api import APIError
from exposed.core.errors import ImportValidationError
from exposed.core.refresh import refresh_members
from tests.fakes import AS_OF, TERM_START, ParliamentFixture
from tests.memory import MemoryStore

HISTORICAL_FILTERS: dict[str, str | int] = {
    "MembershipInDateRange.WasMemberOnOrAfter": "2024-07-04",
    "MembershipInDateRange.WasMemberOnOrBefore": "2026-09-15",
    "MembershipInDateRange.WasMemberOfHouse": 1,
}


def test_search_page_returns_typed_profiles_and_pagination():
    fixture = ParliamentFixture()
    fixture.leave(2, lords=True)
    api = fixture.api()
    with api.client:
        page = api.search_page(HISTORICAL_FILTERS, skip=0, take=100)

    assert page.total_results == 3
    assert page.skip == 0
    members = {member.parliament_member_id: member for member in page.members}
    assert set(members) == {1, 2, 3}
    assert members[2].latest_house == 2
    assert members[2].name == "Example Member 2"
    assert members[2].party_name == "Example party"


def test_history_request_includes_every_id_in_batch():
    fixture = ParliamentFixture(100)
    api = fixture.api()
    with api.client:
        batch = api.histories(set(fixture.profiles))

    histories = {history.parliament_member_id: history for history in batch.histories}
    assert set(histories) == set(range(1, 101))
    previous, current = histories[1].house_memberships
    assert previous.end_date == date(2024, 5, 30)
    assert current.house == 1
    assert current.start_date == date(2024, 7, 4)
    assert current.end_date is None


def test_rate_limit_retries_honor_retry_after():
    fixture = ParliamentFixture(1)
    responses = iter([httpx.Response(429, headers={"Retry-After": "3"}), None])
    fixture.override = lambda _: next(responses)
    api = fixture.api()
    with api.client:
        page = api.search_page(HISTORICAL_FILTERS, skip=0, take=100)

    assert [member.parliament_member_id for member in page.members] == [1]
    assert 3 in fixture.sleeps


def test_persistent_server_failure_stops_after_four_attempts():
    fixture = ParliamentFixture(1)
    fixture.override = lambda _: httpx.Response(503)
    api = fixture.api()
    with api.client, pytest.raises(APIError, match="503"):
        api.search_page(HISTORICAL_FILTERS, skip=0, take=100)
    assert len(fixture.requests) == 4


def test_http_400_is_not_retried():
    fixture = ParliamentFixture(1)
    fixture.override = lambda _: httpx.Response(400)
    api = fixture.api()
    with api.client, pytest.raises(APIError, match="400"):
        api.search_page(HISTORICAL_FILTERS, skip=0, take=100)
    assert len(fixture.requests) == 1


def test_member_source_refreshes_current_and_former_members_across_pages():
    fixture = ParliamentFixture(101)
    fixture.leave(101, lords=True)
    api = fixture.api()

    with api.client:
        result = refresh_members(TERM_START, AS_OF, api, MemoryStore())

    assert (result["members"], result["current_commons"], result["former_commons"]) == (101, 100, 1)
    historical_requests = [
        r for r in fixture.requests if "MembershipInDateRange.WasMemberOnOrAfter" in r.url.params
    ]
    assert [r.url.params["skip"] for r in historical_requests] == ["0", "100"]
    assert all(
        r.url.params["MembershipInDateRange.WasMemberOnOrAfter"] == "2024-07-04"
        and r.url.params["MembershipInDateRange.WasMemberOnOrBefore"] == "2026-09-15"
        for r in historical_requests
    )


@pytest.mark.parametrize("count", [0, 101])
def test_member_source_rejects_invalid_history_batch_sizes_before_http(count):
    fixture = ParliamentFixture(count)
    api = fixture.api()

    with api.client, pytest.raises(ImportValidationError, match="between 1 and 100"):
        api.member_histories(set(fixture.profiles))

    assert fixture.requests == []
