from datetime import date

import pytest

from exposed.adapters.postgres import PostgresStore, connect
from exposed.core.errors import ImportFailed, ImportValidationError, SourceError
from exposed.core.models import CommonsService, Member
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


def test_storage_port_preserves_completed_batches_before_allowing_a_retry(store):
    class FailingSource(MemorySource):
        def commons_candidates(self, term_start, as_of):
            yield self.profiles[:1]
            raise SourceError("Required member source unavailable")

    with pytest.raises(ImportFailed, match="source unavailable"):
        refresh_members(TERM_START, AS_OF, FailingSource(), store)

    retried = refresh_members(TERM_START, AS_OF, MemorySource(), store)
    repeated = refresh_members(TERM_START, AS_OF, MemorySource(), store)

    assert (retried["inserted"], retried["unchanged"]) == (1, 1)
    assert (repeated["inserted"], repeated["unchanged"]) == (0, 2)


def test_storage_port_rejects_a_different_term_without_changing_completed_data(store):
    refresh_members(TERM_START, AS_OF, MemorySource(), store)

    with pytest.raises(ImportFailed, match="another term"):
        refresh_members(date(2024, 7, 9), AS_OF, MemorySource(), store)

    result = refresh_members(TERM_START, AS_OF, MemorySource(), store)
    assert (result["inserted"], result["updated"], result["unchanged"]) == (0, 0, 2)


@pytest.mark.parametrize("failure_type", [ImportValidationError, KeyboardInterrupt])
def test_storage_port_rolls_back_active_batch_on_validation_failure_or_interruption(
    store, failure_type
):
    source = MemorySource()
    refresh_members(TERM_START, AS_OF, source, store)
    member = Member.from_profile(source.profiles[0], is_current_commons=True)
    changed = member.model_copy(update={"name": "Must roll back"})
    service = CommonsService.from_history(
        source.member_histories({1})[0],
        term_start=TERM_START,
        as_of=AS_OF,
        is_current_commons=True,
    )

    with pytest.raises(failure_type), store.refresh_batch(TERM_START) as writer:
        assert writer.write_member(changed, service.periods) == "updated"
        raise failure_type()

    result = refresh_members(TERM_START, AS_OF, source, store)
    assert (result["inserted"], result["updated"], result["unchanged"]) == (0, 0, 2)
