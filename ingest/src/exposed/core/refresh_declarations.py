"""Refresh declarations through source evidence and atomic publication ports."""

import logging
from datetime import date

from exposed.core.declaration_ports import DeclarationSource, DeclarationStore
from exposed.core.declarations import (
    Declaration,
    DeclarationSnapshot,
    RetrievedDeclaration,
    canonical_json,
)
from exposed.core.errors import (
    DeclarationParseError,
    ImportFailed,
    ImportValidationError,
    ParentRequired,
    safe_error,
)

logger = logging.getLogger(__name__)


def refresh_declarations(
    term_start: date,
    source: DeclarationSource,
    store: DeclarationStore,
) -> dict[str, object]:
    try:
        with store.refresh(term_start) as writer:
            members = writer.cohort()
            if not members:
                raise ImportValidationError("No stored Commons cohort for the configured term")
            accepted = 0
            sources = DeclarationSources(source)
            processed: set[int] = set()
            for source_member_id, member_id in members.items():
                for batch in source.declarations(source_member_id):
                    for record in batch:
                        sources.remember(record)
                    for record in batch:
                        source_id = record.source_id
                        if source_id is not None:
                            if source_id in processed:
                                continue
                            processed.add(source_id)
                        try:
                            declaration = sources.resolve(record, source_member_id)
                            snapshot = DeclarationSnapshot.from_record(declaration, record)
                        except DeclarationParseError as exc:
                            logger.error(
                                "Rejected declaration %s (member %s, %s): %s",
                                record.identifier,
                                source_member_id,
                                record.context,
                                exc,
                            )
                            continue
                        writer.write_declaration(member_id, snapshot)
                        accepted += 1
                logger.info("Processed declarations for member %s (uncommitted)", source_member_id)
        return {
            "status": "succeeded",
            "term_start": term_start.isoformat(),
            "members": len(members),
            "declarations": accepted,
        }
    except BaseException as exc:
        raise ImportFailed(safe_error(exc), interrupted=isinstance(exc, KeyboardInterrupt)) from exc


class DeclarationSources:
    """Resolve parent declarations within this refresh, regardless of delivery order."""

    def __init__(self, source: DeclarationSource):
        self.source = source
        self.records: dict[int, RetrievedDeclaration] = {}

    def remember(self, record: RetrievedDeclaration) -> None:
        source_id = record.source_id
        if source_id is None:
            return
        if source_id in self.records:
            if canonical_json(self.records[source_id].payload) != canonical_json(record.payload):
                raise ImportValidationError(f"Conflicting duplicate declaration {source_id}")
            logger.warning("Duplicate declaration %s; identical content collapsed", source_id)
        else:
            self.records[source_id] = record

    def resolve(
        self,
        record: RetrievedDeclaration,
        member_id: int,
        ancestors: frozenset[int] = frozenset(),
    ) -> Declaration:
        draft = self.source.interpret(record)
        if draft.id in ancestors:
            raise DeclarationParseError("parent_id", draft.id, "cyclic parent relationship")
        if draft.member_source_id != member_id:
            raise DeclarationParseError(
                "member_source_id", draft.member_source_id, "belongs to another member"
            )
        try:
            return draft.accept()
        except ParentRequired as required:
            parent_id = required.source_id
            if parent_id not in self.records:
                for parent in self.source.parent_declarations(member_id, parent_id):
                    self.remember(parent)
            if parent_id not in self.records:
                raise DeclarationParseError(
                    "parent_id", parent_id, "parent was not returned"
                ) from required
            parent = self.resolve(self.records[parent_id], member_id, ancestors | {draft.id})
            return draft.accept(parent=parent)
