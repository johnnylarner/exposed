from datetime import date

import pytest

from exposed.models import ImportValidationError, parse_member, service_periods
from tests.fakes import AS_OF, TERM_START, ParliamentFixture, service


def test_latest_profile_can_be_lords_while_commons_service_is_retained():
    fixture = ParliamentFixture(1)
    fixture.leave(1, lords=True)
    member = parse_member(fixture.profiles[1], current=False)
    periods = service_periods(1, fixture.histories[1], TERM_START, AS_OF, current=False)
    assert member.latest_house == 2
    assert not member.is_current_commons
    assert len(periods) == 1
    assert periods[0].served_until == date(2025, 3, 17)


def test_long_continuous_source_period_is_clipped_without_losing_original_date():
    history = {"houseMembershipHistory": [service("1987-06-11")]}
    period = service_periods(1, history, TERM_START, AS_OF, current=True)[0]
    assert period.served_from == TERM_START
    assert period.source_start_date == date(1987, 6, 11)


def test_by_election_service_starts_on_its_source_date():
    history = {"houseMembershipHistory": [service("2025-05-01")]}
    period = service_periods(1, history, TERM_START, AS_OF, current=True)[0]
    assert period.served_from == date(2025, 5, 1)


def test_reentry_retains_a_real_service_gap():
    history = {"houseMembershipHistory": [service(end="2025-01-01"), service("2025-06-01")]}
    periods = service_periods(1, history, TERM_START, AS_OF, current=True)
    assert len(periods) == 2
    assert periods[0].served_until == date(2025, 1, 1)
    assert periods[1].served_from == date(2025, 6, 1)


def test_disagreement_between_history_and_current_cohort_is_rejected():
    history = {"houseMembershipHistory": [service(end="2025-01-01")]}
    with pytest.raises(ImportValidationError, match="disagrees"):
        service_periods(1, history, TERM_START, AS_OF, current=True)


@pytest.mark.parametrize("end", ["2024-05-30", "2024-07-04"])
def test_service_that_ceased_before_or_on_term_start_is_excluded(end):
    history = {"houseMembershipHistory": [service("2019-12-12", end)]}
    assert service_periods(1, history, TERM_START, AS_OF, current=False) == []


@pytest.mark.parametrize("bad_date", [None, "nonsense", 20240704])
def test_missing_or_invalid_service_start_fails(bad_date):
    entry = service()
    entry["membershipStartDate"] = bad_date
    with pytest.raises(ImportValidationError, match="invalid date"):
        service_periods(1, {"houseMembershipHistory": [entry]}, TERM_START, AS_OF, current=True)
