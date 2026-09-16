from datetime import date

import pytest

from exposed.adapters.postgres import PostgresStore, connect
from exposed.core.errors import ImportFailed, SourceError
from exposed.core.refresh import refresh_members
from tests.fakes import AS_OF, TERM_START
from tests.memory import MemorySource, MemoryStore


@pytest.fixture(params=["memory", pytest.param("postgres", marks=pytest.mark.integration)])
def store(request):
    if request.param == "memory":
        yield MemoryStore()
    else:
        with connect(request.getfixturevalue("database_url")) as conn:
            yield PostgresStore(conn)


def test_storage_port_rolls_back_a_partial_refresh_before_allowing_a_retry(store):
    class FailingSource(MemorySource):
        def commons_candidates(self, term_start, as_of):
            yield self.profiles[:1]
            raise SourceError("Required member source unavailable")

    with pytest.raises(ImportFailed, match="source unavailable"):
        refresh_members(TERM_START, AS_OF, FailingSource(), store)

    retried = refresh_members(TERM_START, AS_OF, MemorySource(), store)
    repeated = refresh_members(TERM_START, AS_OF, MemorySource(), store)

    assert (retried["inserted"], retried["unchanged"]) == (2, 0)
    assert (repeated["inserted"], repeated["unchanged"]) == (0, 2)


def test_storage_port_rejects_a_different_term_without_changing_completed_data(store):
    refresh_members(TERM_START, AS_OF, MemorySource(), store)

    with pytest.raises(ImportFailed, match="another term"):
        refresh_members(date(2024, 7, 9), AS_OF, MemorySource(), store)

    result = refresh_members(TERM_START, AS_OF, MemorySource(), store)
    assert (result["inserted"], result["updated"], result["unchanged"]) == (0, 0, 2)
