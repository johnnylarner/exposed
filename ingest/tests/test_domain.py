from datetime import date

from exposed.core.models import CommonsService, HouseMembership, MemberHistory


def test_commons_service_can_be_constructed_from_calendar_dates_without_source_json():
    history = MemberHistory(
        parliament_member_id=7,
        house_memberships=(HouseMembership(house=1, start_date=date(1987, 6, 11)),),
    )

    service = CommonsService.from_history(
        history,
        term_start=date(2024, 7, 4),
        as_of=date(2026, 9, 15),
        is_current_commons=True,
    )

    assert [(p.source_start_date, p.served_from, p.served_until) for p in service.periods] == [
        (date(1987, 6, 11), date(2024, 7, 4), None)
    ]
