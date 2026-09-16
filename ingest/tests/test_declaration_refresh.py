from decimal import Decimal
from uuid import UUID

from exposed.core.declarations import DeclarationDraft, FundingEntry
from exposed.core.refresh_declarations import refresh_declarations
from tests.declaration_memory import MemoryDeclarationSource, MemoryDeclarationStore
from tests.fakes import TERM_START


def test_refresh_resolves_parent_and_publishes_parsed_fields_without_infrastructure():
    source = MemoryDeclarationSource()
    child = source.add(
        DeclarationDraft(
            id=101,
            member_source_id=1,
            category_id=1,
            category_name="Payments",
            parent_id=500,
            payer=None,
            funding=(
                FundingEntry(
                    funder=None, amount=Decimal("340"), currency="GBP", payment_type="Monetary"
                ),
            ),
        )
    )
    source.add(
        DeclarationDraft(
            id=500,
            member_source_id=1,
            category_id=12,
            category_name="Employment",
            payer="Publisher",
            funding=(),
        ),
        publish=False,
    )
    store = MemoryDeclarationStore(TERM_START, {1: UUID(int=1)})

    assert refresh_declarations(TERM_START, source, store) == {
        "status": "succeeded",
        "term_start": "2024-07-04",
        "members": 1,
        "declarations": 1,
    }
    declaration, fetched_at = store.records[101]
    assert fetched_at == child.fetched_at
    assert declaration.funding[0].funder == "Publisher"


def test_item_rejection_preserves_previous_declaration_and_commits_valid_siblings(caplog):
    from exposed.core.errors import DeclarationParseError

    source = MemoryDeclarationSource()
    original = DeclarationDraft(
        id=101,
        member_source_id=1,
        category_id=9,
        category_name="Miscellaneous",
        payer=None,
        funding=(),
    )
    first = source.add(original)
    store = MemoryDeclarationStore(TERM_START, {1: UUID(int=1)})
    refresh_declarations(TERM_START, source, store)
    before = store.records[101]
    source.drafts[101] = DeclarationParseError("funding.amount", "janky", "unsupported amount")
    good = source.add(original.model_copy(update={"id": 102}))
    rejected = source.add(original.model_copy(update={"id": 103}))
    source.drafts[103] = DeclarationParseError("funding.amount", "janky", "unsupported amount")
    source.member_batches[1] = [(first, rejected, good)]

    result = refresh_declarations(TERM_START, source, store)

    assert result["status"] == "succeeded"
    assert result["declarations"] == 1
    assert store.records[101] == before
    assert set(store.records) == {101, 102}
    assert "Rejected declaration 101" in caplog.text
    assert "janky" in caplog.text


def test_conflicting_duplicate_rolls_back_earlier_batches(caplog):
    import pytest

    from exposed.core.errors import ImportFailed

    source = MemoryDeclarationSource()
    record = source.add(
        DeclarationDraft(
            id=101,
            member_source_id=1,
            category_id=9,
            category_name="Miscellaneous",
            payer=None,
            funding=(),
        )
    )
    store = MemoryDeclarationStore(TERM_START, {1: UUID(int=1)})
    source.member_batches[1].append((record,))
    assert refresh_declarations(TERM_START, source, store)["declarations"] == 1
    assert "Duplicate declaration 101" in caplog.text
    before = dict(store.records)
    new = source.add(
        DeclarationDraft(
            id=102,
            member_source_id=1,
            category_id=9,
            category_name="Miscellaneous",
            payer=None,
            funding=(),
        )
    )
    source.member_batches[1] = [(new, record), (record.model_copy(update={"payload": False}),)]
    with pytest.raises(ImportFailed, match="Conflicting duplicate declaration 101"):
        refresh_declarations(TERM_START, source, store)
    assert store.records == before


def test_storage_failure_rolls_back_and_preserves_core_diagnostic_cause():
    from contextlib import contextmanager

    import pytest

    from exposed.core.errors import ImportFailed, StorageError

    failure = StorageError("publication failed")

    class FailingStore(MemoryDeclarationStore):
        @contextmanager
        def refresh(self, term_start):
            with super().refresh(term_start) as writer:
                yield writer
                raise failure

    store = FailingStore(TERM_START, {1: UUID(int=1)})
    source = MemoryDeclarationSource()
    source.add(
        DeclarationDraft(
            id=101,
            member_source_id=1,
            category_id=9,
            category_name="Miscellaneous",
            payer=None,
            funding=(),
        )
    )
    with pytest.raises(ImportFailed, match="publication failed") as error:
        refresh_declarations(TERM_START, source, store)
    assert error.value.__cause__ is failure
    assert store.records == {}
