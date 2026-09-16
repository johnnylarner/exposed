from decimal import Decimal
from uuid import UUID

import pytest

from exposed.adapters.declaration_postgres import PostgresDeclarationStore
from exposed.adapters.postgres import connect
from exposed.core.declarations import DeclarationDraft, FundingEntry
from exposed.core.errors import ImportFailed, SourceError
from exposed.core.refresh_declarations import refresh_declarations
from tests.declaration_memory import MemoryDeclarationSource, MemoryDeclarationStore
from tests.fakes import TERM_START, ParliamentFixture
from tests.test_importer import run as import_members


@pytest.fixture(params=["memory", pytest.param("postgres", marks=pytest.mark.integration)])
def declaration_store(request):
    if request.param == "memory":
        store = MemoryDeclarationStore(TERM_START, {1: UUID(int=1), 2: UUID(int=2)})
        yield (
            store,
            lambda: {id: d.funding[0].amount for id, (d, _) in store.records.items()},
        )
    else:
        url = request.getfixturevalue("database_url")
        import_members(url, ParliamentFixture(2))

        def amounts():
            with connect(url) as reader:
                return {
                    r["source_declaration_id"]: r["amount"]
                    for r in reader.execute(
                        "SELECT source_declaration_id, amount FROM exposed.funding_entries"
                    )
                }

        with connect(url) as conn:
            yield PostgresDeclarationStore(conn), amounts


def test_declaration_store_publishes_member_writes_together_and_rolls_back_before_retry(
    declaration_store,
):
    store, amounts = declaration_store
    source = MemoryDeclarationSource()
    source.add(
        DeclarationDraft(
            id=101,
            member_source_id=1,
            category_id=3,
            category_name="Donations",
            payer="Donor",
            funding=(
                FundingEntry(
                    funder="Donor", amount=Decimal("100"), currency="GBP", payment_type=None
                ),
            ),
        )
    )
    refresh_declarations(TERM_START, source, store)
    assert amounts() == {101: Decimal("100")}
    old_draft = source.drafts[101]
    assert isinstance(old_draft, DeclarationDraft)
    new_draft = DeclarationDraft(
        **{
            **old_draft.model_dump(),
            "funding": (
                FundingEntry(
                    funder="Donor", amount=Decimal("200"), currency="GBP", payment_type=None
                ),
            ),
        }
    )

    class FailingSource(MemoryDeclarationSource):
        def declarations(self, member_id):
            yield from super().declarations(member_id)
            if member_id == 1:
                assert amounts() == {101: Decimal("100")}
                raise SourceError("later source failure")

    failing = FailingSource()
    failing.add(new_draft)
    with pytest.raises(ImportFailed, match="later source failure"):
        refresh_declarations(TERM_START, failing, store)
    assert amounts() == {101: Decimal("100")}
    retry = MemoryDeclarationSource()
    retry.add(new_draft)
    refresh_declarations(TERM_START, retry, store)
    assert amounts() == {101: Decimal("200")}


@pytest.mark.parametrize("failure", [SourceError("later member failed"), KeyboardInterrupt()])
def test_completed_member_is_visible_and_survives_later_member_failure(declaration_store, failure):
    store, amounts = declaration_store

    class FailingSource(MemoryDeclarationSource):
        def declarations(self, member_id):
            if member_id == 2:
                assert amounts() == {101: Decimal("100")}
            yield from super().declarations(member_id)
            if member_id == 2:
                assert amounts() == {101: Decimal("100")}
                raise failure

    source = FailingSource()
    for member in (1, 2):
        source.add(
            DeclarationDraft(
                id=100 + member,
                member_source_id=member,
                category_id=3,
                category_name="Donations",
                payer="Donor",
                funding=(
                    FundingEntry(
                        funder="Donor", amount=Decimal("100"), currency="GBP", payment_type=None
                    ),
                ),
            )
        )
    with pytest.raises(ImportFailed) as error:
        refresh_declarations(TERM_START, source, store)
    assert error.value.__cause__ is failure
    assert error.value.interrupted == isinstance(failure, KeyboardInterrupt)
    assert amounts() == {101: Decimal("100")}
