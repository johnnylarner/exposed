import json
from datetime import date

import pytest
from pydantic import ValidationError

from exposed.models import (
    CommonsService,
    HistoryBatch,
    Member,
    MemberHistory,
    MemberResponse,
    SearchPage,
    ServicePeriod,
    validation_error_message,
)
from tests.fakes import AS_OF, TERM_START, ParliamentFixture, service


def search_payload(profile: dict[str, object]) -> str:
    return json.dumps({"items": [{"value": profile}], "totalResults": 1, "skip": 0})


def test_search_response_ignores_unused_fields_at_every_level():
    profile = ParliamentFixture(1).profiles[1]
    profile["latestParty"]["unused"] = {"unexpected": [1, 2, 3]}
    profile["latestHouseMembership"]["membershipStartDate"] = "unused invalid date"
    page = SearchPage.from_json(search_payload(profile))
    assert page.members[0].name == "Example Member 1"
    assert page.members[0].party_id == 1
    assert "unknownFutureField" not in page.model_dump_json()


@pytest.mark.parametrize("bad_id", [None, True, False, "1", 1.0, 0, -1])
def test_profile_ids_are_strict_positive_integers(bad_id):
    profile = ParliamentFixture(1).profiles[1]
    profile["id"] = bad_id
    with pytest.raises(ValidationError) as error:
        SearchPage.from_json(search_payload(profile))
    assert error.value.errors()[0]["loc"] == ("items", 0, "value", "id")


@pytest.mark.parametrize("bad_house", [None, True, "1", 1.0, 0, 3])
def test_house_is_a_strict_integer_restricted_to_commons_or_lords(bad_house):
    profile = ParliamentFixture(1).profiles[1]
    profile["latestHouseMembership"]["house"] = bad_house
    with pytest.raises(ValidationError, match="house"):
        SearchPage.from_json(search_payload(profile))


@pytest.mark.parametrize("name", [None, "", " \t\n", 123])
def test_display_name_must_be_a_nonblank_string(name):
    profile = ParliamentFixture(1).profiles[1]
    profile["nameDisplayAs"] = name
    with pytest.raises(ValidationError, match="nameDisplayAs"):
        SearchPage.from_json(search_payload(profile))


@pytest.mark.parametrize("party", [None, {}])
def test_nullable_and_missing_optional_profile_fields(party):
    profile = ParliamentFixture(1).profiles[1]
    profile["latestParty"] = party
    profile["latestHouseMembership"].pop("membershipFrom")
    member = SearchPage.from_json(search_payload(profile)).members[0]
    assert member.party_id is None
    assert member.party_name is None
    assert member.latest_membership_from is None
    profile.pop("latestParty")
    assert SearchPage.from_json(search_payload(profile)).members[0] == member


@pytest.mark.parametrize("party", [False, 0, "", []])
def test_malformed_party_is_not_treated_as_missing(party):
    profile = ParliamentFixture(1).profiles[1]
    profile["latestParty"] = party
    with pytest.raises(ValidationError, match="latestParty"):
        SearchPage.from_json(search_payload(profile))


@pytest.mark.parametrize("field", ["id", "nameDisplayAs", "latestHouseMembership"])
def test_missing_required_profile_fields_are_rejected(field):
    profile = ParliamentFixture(1).profiles[1]
    profile.pop(field)
    with pytest.raises(ValidationError, match=field):
        SearchPage.from_json(search_payload(profile))


@pytest.mark.parametrize("field", ["skip", "totalResults"])
@pytest.mark.parametrize("value", [None, True, "0", 0.0, -1])
def test_pagination_fields_are_strict_nonnegative_integers(field, value):
    payload = {"items": [], "totalResults": 0, "skip": 0, field: value}
    with pytest.raises(ValidationError, match=field):
        SearchPage.from_json(json.dumps(payload))


@pytest.mark.parametrize(
    "timestamp",
    [
        "2024-07-04",
        "2024-07-04T00:00:00",
        "2024-07-04T12:34:56.123456Z",
        "2024-07-04T00:15:00+02:00",
        "2024-07-04T23:15:00-04:00",
    ],
)
def test_source_timestamps_become_calendar_dates_without_timezone_conversion(timestamp):
    entry = {**service(), "membershipStartDate": timestamp, "membershipEndDate": timestamp}
    batch = HistoryBatch.from_json(
        json.dumps([{"value": {"id": 1, "houseMembershipHistory": [entry]}}])
    )
    membership = batch.histories[0].house_memberships[0]
    assert membership.start_date == date(2024, 7, 4)
    assert membership.end_date == date(2024, 7, 4)
    assert membership.model_dump(mode="json") == {
        "house": 1,
        "start_date": "2024-07-04",
        "end_date": "2024-07-04",
    }


@pytest.mark.parametrize("bad_date", [None, "nonsense", "2024-02-30", 20240704, True])
def test_missing_or_invalid_service_start_fails_with_a_field_path(bad_date):
    entry = {**service(), "membershipStartDate": bad_date}
    with pytest.raises(ValidationError) as error:
        HistoryBatch.from_json(
            json.dumps([{"value": {"id": 1, "houseMembershipHistory": [entry]}}])
        )
    message = validation_error_message(error.value)
    assert "0.value.houseMembershipHistory.0.membershipStartDate" in message
    assert str(bad_date) not in message


def test_open_ended_service_accepts_missing_or_null_end_dates():
    entry = service()
    payload = {"id": 1, "houseMembershipHistory": [entry]}
    with_null = HistoryBatch.from_json(json.dumps([{"value": payload}]))
    entry.pop("membershipEndDate")
    with_missing = HistoryBatch.from_json(json.dumps([{"value": payload}]))
    assert with_null == with_missing
    assert with_missing.histories[0].house_memberships[0].end_date is None


@pytest.mark.parametrize("entries", [None, {}, []])
def test_history_requires_a_nonempty_collection_of_memberships(entries):
    with pytest.raises(ValidationError, match="houseMembershipHistory"):
        HistoryBatch.from_json(
            json.dumps([{"value": {"id": 1, "houseMembershipHistory": entries}}])
        )


def test_response_models_preserve_duplicate_ids_for_the_importer():
    fixture = ParliamentFixture(1)
    page = SearchPage.from_json(
        json.dumps(
            {
                "items": [{"value": fixture.profiles[1]}] * 2,
                "totalResults": 2,
                "skip": 0,
            }
        )
    )
    batch = HistoryBatch.from_json(json.dumps([{"value": fixture.histories[1]}] * 2))
    assert len(page.members) == 2
    assert len(batch.histories) == 2


@pytest.mark.parametrize("payload", ["not json", "null", "[]", '{"items": [null]}'])
def test_invalid_search_envelopes_fail_validation(payload):
    with pytest.raises(ValidationError):
        SearchPage.from_json(payload)


@pytest.mark.parametrize("payload", ["not json", "null", "{}", "[null]"])
def test_invalid_history_envelopes_fail_validation(payload):
    with pytest.raises(ValidationError):
        HistoryBatch.from_json(payload)


def test_member_requires_boolean_current_commons_status():
    with pytest.raises(ValidationError, match="is_current_commons"):
        Member.model_validate(
            {
                "parliament_member_id": 1,
                "name": "Example",
                "latest_house": 1,
                "is_current_commons": "true",
            }
        )


def test_service_period_serializes_calendar_dates():
    period = ServicePeriod(
        parliament_member_id=1, source_start_date=date(1987, 6, 11), served_from=date(2024, 7, 4)
    )
    assert period.model_dump(mode="json")["served_from"] == "2024-07-04"


def test_service_period_rejects_boolean_member_id():
    with pytest.raises(ValidationError, match="parliament_member_id"):
        ServicePeriod.model_validate(
            {
                "parliament_member_id": True,
                "source_start_date": date(1987, 6, 11),
                "served_from": date(2024, 7, 4),
            }
        )


def test_latest_profile_can_be_lords_while_commons_service_is_retained():
    fixture = ParliamentFixture(1)
    fixture.leave(1, lords=True)
    member = Member.from_profile(
        MemberResponse.model_validate(fixture.profiles[1]).to_profile(), is_current_commons=False
    )
    periods = CommonsService.from_history(
        MemberHistory.model_validate_json(json.dumps(fixture.histories[1])),
        term_start=TERM_START,
        as_of=AS_OF,
        is_current_commons=False,
    ).periods
    assert member.latest_house == 2
    assert not member.is_current_commons
    assert len(periods) == 1
    assert periods[0].served_until == date(2025, 3, 17)


def test_long_continuous_source_period_is_clipped_without_losing_original_date():
    history = {"houseMembershipHistory": [service("1987-06-11")]}
    period = CommonsService.from_history(
        MemberHistory.model_validate_json(json.dumps({"id": 1, **history})),
        term_start=TERM_START,
        as_of=AS_OF,
        is_current_commons=True,
    ).periods[0]
    assert period.served_from == TERM_START
    assert period.source_start_date == date(1987, 6, 11)


def test_by_election_service_starts_on_its_source_date():
    history = {"houseMembershipHistory": [service("2025-05-01")]}
    period = CommonsService.from_history(
        MemberHistory.model_validate_json(json.dumps({"id": 1, **history})),
        term_start=TERM_START,
        as_of=AS_OF,
        is_current_commons=True,
    ).periods[0]
    assert period.served_from == date(2025, 5, 1)


def test_reentry_retains_a_real_service_gap():
    history = {"houseMembershipHistory": [service(end="2025-01-01"), service("2025-06-01")]}
    periods = CommonsService.from_history(
        MemberHistory.model_validate_json(json.dumps({"id": 1, **history})),
        term_start=TERM_START,
        as_of=AS_OF,
        is_current_commons=True,
    ).periods
    assert len(periods) == 2
    assert periods[0].served_until == date(2025, 1, 1)
    assert periods[1].served_from == date(2025, 6, 1)


def test_disagreement_between_history_and_current_cohort_is_rejected():
    history = {"houseMembershipHistory": [service(end="2025-01-01")]}
    with pytest.raises(ValidationError, match="disagrees"):
        CommonsService.from_history(
            MemberHistory.model_validate_json(json.dumps({"id": 1, **history})),
            term_start=TERM_START,
            as_of=AS_OF,
            is_current_commons=True,
        )


@pytest.mark.parametrize("end", ["2024-05-30", "2024-07-04"])
def test_service_that_ceased_before_or_on_term_start_is_excluded(end):
    history = {"houseMembershipHistory": [service("2019-12-12", end)]}
    periods = CommonsService.from_history(
        MemberHistory.model_validate_json(json.dumps({"id": 1, **history})),
        term_start=TERM_START,
        as_of=AS_OF,
        is_current_commons=False,
    ).periods

    assert periods == ()


@pytest.mark.parametrize(
    ("memberships", "message"),
    [
        ([service("2025-06-01", "2025-05-01")], "ends before"),
        ([service(), service(end="2025-05-01")], "conflicting"),
        ([service(), service("2025-05-01")], "overlapping"),
        ([service("2024-07-04", "2025-06-01"), service("2025-05-01")], "overlapping"),
    ],
)
def test_invalid_service_periods_are_rejected(memberships, message):
    history = MemberHistory.model_validate_json(
        json.dumps({"id": 1, "houseMembershipHistory": memberships})
    )
    with pytest.raises(ValidationError, match=message):
        CommonsService.from_history(
            history, term_start=TERM_START, as_of=AS_OF, is_current_commons=True
        )


def test_identical_source_periods_are_deduplicated():
    history = MemberHistory.model_validate_json(
        json.dumps({"id": 1, "houseMembershipHistory": [service(), service()]})
    )
    periods = CommonsService.from_history(
        history, term_start=TERM_START, as_of=AS_OF, is_current_commons=True
    ).periods

    assert len(periods) == 1


def test_current_member_requires_commons_profile():
    fixture = ParliamentFixture(1)
    fixture.leave(1, lords=True)
    profile = MemberResponse.model_validate(fixture.profiles[1]).to_profile()
    with pytest.raises(ValidationError, match="inconsistent latest House"):
        Member.from_profile(profile, is_current_commons=True)
