from collections.abc import Iterator
from datetime import date

import pytest

from exposed.core.errors import ImportFailed, SourceError
from exposed.core.models import MemberProfile
from exposed.core.refresh import refresh_members
from tests.fakes import AS_OF, TERM_START
from tests.memory import MemorySource, MemoryStore


def test_refresh_reports_current_and_former_members_without_http_or_postgresql():
    store = MemoryStore()
    source = MemorySource()

    first = refresh_members(TERM_START, AS_OF, source, store)
    second = refresh_members(TERM_START, AS_OF, source, store)

    assert first == {
        "status": "succeeded",
        "term_start": "2024-07-04",
        "as_of": "2026-09-15",
        "members": 2,
        "current_commons": 1,
        "former_commons": 1,
        "inserted": 2,
        "updated": 0,
        "unchanged": 0,
        "service_periods": 2,
        "excluded_candidates": 0,
    }
    assert (second["inserted"], second["unchanged"]) == (0, 2)


def test_later_source_failure_preserves_the_completed_refresh_and_its_diagnostic_cause():
    failure = SourceError("Members API returned HTTP 503 at /api/Members/Search")

    class FailingSource(MemorySource):
        def commons_candidates(
            self, term_start: date, as_of: date
        ) -> Iterator[tuple[MemberProfile, ...]]:
            yield (MemberProfile(parliament_member_id=1, name="Unpublished", latest_house=1),)
            raise failure

    store = MemoryStore()
    refresh_members(TERM_START, AS_OF, MemorySource(), store)

    with pytest.raises(ImportFailed, match="HTTP 503") as error:
        refresh_members(TERM_START, AS_OF, FailingSource(), store)

    assert error.value.__cause__ is failure
    recovered = refresh_members(TERM_START, AS_OF, MemorySource(), store)
    assert (recovered["updated"], recovered["unchanged"]) == (0, 2)
