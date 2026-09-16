"""Source evidence and atomic publication contracts for declaration refreshes."""

from collections.abc import Iterator, Mapping
from contextlib import AbstractContextManager
from datetime import date, datetime
from typing import Protocol
from uuid import UUID

from exposed.core.declarations import Declaration, DeclarationDraft, RetrievedDeclaration


class DeclarationSource(Protocol):
    """Return all available declarations for a stored member, including expired ones.

    Stream bounded batches in source order, retaining duplicates and malformed items.
    Evidence is opaque to the use case: interpret it only after duplicate checks.
    Translate an uninterpretable item into DeclarationParseError with source field
    paths and original values. A request/envelope failure raises SourceError or
    ImportValidationError and fails the whole refresh. Preserve diagnostic causes.
    Retrieval timestamps and source-specific request context belong to the adapter.
    """

    def declarations(self, member_id: int) -> Iterator[tuple[RetrievedDeclaration, ...]]: ...

    def parent_declarations(
        self, member_id: int, parent_id: int
    ) -> tuple[RetrievedDeclaration, ...]:
        """Return parent evidence, retaining duplicates; absence returns an empty tuple."""
        ...

    def interpret(self, record: RetrievedDeclaration) -> DeclarationDraft: ...


class DeclarationWriter(Protocol):
    """Stage accepted declarations in the active member transaction.

    Do not create or change member/service/term records. Upsert by source ID,
    keeping declaration UUIDs and unchanged funding UUIDs. Compare funding values
    with multiplicity, ignoring order; replace changed groups in full. Store parsed
    declaration fields, registration date, funding and retrieval time together; leave unmentioned
    declarations intact. Source responses are not persisted.
    """

    def write_declaration(
        self, member_id: UUID, declaration: Declaration, fetched_at: datetime
    ) -> None: ...


class DeclarationStore(Protocol):
    """Read the cohort once and publish accepted declarations atomically per member.

    Publish on successful exit; roll back on any exception, including interruptions
    and late source failures for that member. Completed members stay committed.
    Translate driver failures into StorageError with the original cause.
    Other readers see only committed data. Execution is externally
    controlled; this contract adds no locks or concurrent-run coordination.
    """

    def cohort(self, term_start: date) -> Mapping[int, UUID]:
        """Return distinct current/former members with Commons service in this term."""
        ...

    def refresh_member(self) -> AbstractContextManager[DeclarationWriter]: ...
