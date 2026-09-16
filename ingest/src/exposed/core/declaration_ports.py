"""Source evidence and atomic publication contracts for declaration refreshes."""

from collections.abc import Iterator, Mapping
from contextlib import AbstractContextManager
from datetime import date
from typing import Protocol
from uuid import UUID

from exposed.core.declarations import DeclarationDraft, DeclarationSnapshot, RetrievedDeclaration


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
    """Read the stored cohort and stage accepted declarations in the active refresh.

    Include distinct current/former members with Commons service in the configured
    term; do not create or change member/service/term records. Upsert by source ID,
    keeping declaration UUIDs and unchanged funding UUIDs. Compare funding values
    with multiplicity, ignoring order; replace changed groups in full. Store each
    accepted payload and projection together; leave unmentioned declarations intact.
    """

    def cohort(self) -> Mapping[int, UUID]: ...

    def write_declaration(self, member_id: UUID, snapshot: DeclarationSnapshot) -> None: ...


class DeclarationStore(Protocol):
    """One atomic scope for cohort reads and all accepted declaration writes.

    Publish on successful exit; roll back on any exception, including interruptions
    and late source failures. Translate driver failures into StorageError with the
    original cause. Other readers see only committed data. Execution is externally
    controlled; this contract adds no locks or concurrent-run coordination.
    """

    def refresh(self, term_start: date) -> AbstractContextManager[DeclarationWriter]: ...
