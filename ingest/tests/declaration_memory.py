"""Declaration ports with opaque synthetic evidence and atomic in-memory publication."""

from collections.abc import Iterator
from contextlib import contextmanager
from datetime import UTC, date, datetime
from uuid import UUID

from exposed.core.declaration_ports import DeclarationWriter
from exposed.core.declarations import Declaration, DeclarationDraft, RetrievedDeclaration
from exposed.core.errors import DeclarationParseError


class MemoryDeclarationSource:
    def __init__(self):
        self.records: dict[int, RetrievedDeclaration] = {}
        self.drafts: dict[int, DeclarationDraft | DeclarationParseError] = {}
        self.member_batches: dict[int, list[tuple[RetrievedDeclaration, ...]]] = {}

    def add(self, draft: DeclarationDraft, *, publish: bool = True) -> RetrievedDeclaration:
        record = RetrievedDeclaration(
            identifier=draft.id,
            payload={"evidence": f"source record {draft.id}"},
            fetched_at=datetime(2026, 9, 16, tzinfo=UTC),
            context="synthetic source",
        )
        self.records[draft.id] = record
        self.drafts[draft.id] = draft
        if publish:
            self.member_batches.setdefault(draft.member_source_id, []).append((record,))
        return record

    def declarations(self, member_id: int) -> Iterator[tuple[RetrievedDeclaration, ...]]:
        yield from self.member_batches.get(member_id, [])

    def parent_declarations(
        self, member_id: int, parent_id: int
    ) -> tuple[RetrievedDeclaration, ...]:
        record = self.records.get(parent_id)
        return () if record is None else (record,)

    def interpret(self, record: RetrievedDeclaration) -> DeclarationDraft:
        assert record.source_id is not None
        result = self.drafts[record.source_id]
        if isinstance(result, DeclarationParseError):
            raise result
        return result


class MemoryDeclarationWriter:
    def __init__(self, members: dict[int, UUID], records: dict[int, tuple[Declaration, datetime]]):
        self.members, self.records = members, records

    def cohort(self) -> dict[int, UUID]:
        return dict(self.members)

    def write_declaration(
        self, member_id: UUID, declaration: Declaration, fetched_at: datetime
    ) -> None:
        assert member_id == self.members[declaration.member_source_id]
        self.records[declaration.id] = (declaration, fetched_at)


class MemoryDeclarationStore:
    def __init__(self, term_start: date, members: dict[int, UUID]):
        self.term_start, self.members = term_start, members
        self.records: dict[int, tuple[Declaration, datetime]] = {}

    @contextmanager
    def refresh(self, term_start: date) -> Iterator[DeclarationWriter]:
        pending = dict(self.records)
        members = self.members if term_start == self.term_start else {}
        yield MemoryDeclarationWriter(members, pending)
        self.records = pending
