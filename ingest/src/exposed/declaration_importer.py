"""Refresh source declarations independently of member ingestion."""

import logging
from datetime import UTC, date, datetime

from pydantic import JsonValue, ValidationError

from exposed.declaration_api import DeclarationsAPI
from exposed.declaration_db import cohort, write_declaration
from exposed.declaration_models import (
    Declaration,
    DeclarationParseError,
    ParentRequired,
    SourceDeclaration,
    canonical_json,
)
from exposed.importer import BATCH_SIZE, ImportFailed, connect, safe_error
from exposed.models import ImportValidationError

logger = logging.getLogger(__name__)


def run_import(database_url: str, term_start: date, api: DeclarationsAPI) -> dict[str, object]:
    try:
        with connect(database_url) as conn, conn.transaction():
            members = cohort(conn, term_start)
            if not members:
                raise ImportValidationError("No stored Commons cohort for the configured term")
            accepted = 0
            sources = DeclarationSources(api)
            processed: set[int] = set()
            for source_member_id, member_id in members.items():
                offset = 0
                while True:
                    page = api.search_page(source_member_id, skip=offset, take=BATCH_SIZE)
                    fetched_at = datetime.now(UTC)
                    for source in page.items:
                        sources.remember(source)
                    for source in page.items:
                        source_id = source.get("id") if isinstance(source, dict) else None
                        if type(source_id) is int:
                            if source_id in processed:
                                continue
                            processed.add(source_id)
                        try:
                            declaration = sources.resolve(source, source_member_id)
                        except (ValidationError, DeclarationParseError) as exc:
                            source_id = source.get("id") if isinstance(source, dict) else None
                            logger.error(
                                "Rejected declaration %s (member %s, offset %s): %s",
                                source_id,
                                source_member_id,
                                offset,
                                parsing_error(exc),
                            )
                            continue
                        write_declaration(conn, member_id, declaration, source, fetched_at)
                        accepted += 1
                    offset += len(page.items)
                    if not page.items or offset >= page.total_results:
                        break
                logger.info("Processed declarations for member %s (uncommitted)", source_member_id)
        return {
            "status": "succeeded",
            "term_start": term_start.isoformat(),
            "members": len(members),
            "declarations": accepted,
        }
    except BaseException as exc:
        raise ImportFailed(safe_error(exc), interrupted=isinstance(exc, KeyboardInterrupt)) from exc


def parsing_error(exc: ValidationError | DeclarationParseError) -> str:
    if isinstance(exc, DeclarationParseError):
        return str(exc)
    return "; ".join(
        f"{'.'.join(str(part) for part in issue['loc']) or 'declaration'}: "
        f"{issue['msg']}; input={issue.get('input')!r}"
        for issue in exc.errors(include_url=False, include_context=False)
    )


class DeclarationSources:
    """Resolve parent source objects within this refresh, regardless of page order."""

    def __init__(self, api: DeclarationsAPI):
        self.api = api
        self.sources: dict[int, JsonValue] = {}

    def remember(self, source: JsonValue) -> None:
        if isinstance(source, dict) and type(source.get("id")) is int:
            source_id = source["id"]
            if isinstance(source_id, int):
                if source_id in self.sources:
                    if canonical_json(self.sources[source_id]) != canonical_json(source):
                        raise ImportValidationError(
                            f"Conflicting duplicate declaration {source_id}"
                        )
                    logger.warning(
                        "Duplicate declaration %s; identical content collapsed", source_id
                    )
                else:
                    self.sources[source_id] = source

    def resolve(
        self, raw: JsonValue, member_id: int, ancestors: frozenset[int] = frozenset()
    ) -> Declaration:
        source = SourceDeclaration.model_validate(raw)
        if source.id in ancestors:
            raise DeclarationParseError("parentInterestId", source.id, "cyclic parent relationship")
        if source.registrant.member.id != member_id:
            raise DeclarationParseError(
                "registrant.memberDetail.id",
                source.registrant.member.id,
                "belongs to another member",
            )
        try:
            return Declaration.from_source(source)
        except ParentRequired as required:
            parent_id = required.source_id
            if parent_id not in self.sources:
                page = self.api.search_page(
                    member_id, skip=0, take=BATCH_SIZE, interest_id=parent_id
                )
                for item in page.items:
                    self.remember(item)
            if parent_id not in self.sources:
                raise DeclarationParseError(
                    "parentInterestId", parent_id, "parent was not returned"
                ) from required
            parent = self.resolve(self.sources[parent_id], member_id, ancestors | {source.id})
            return Declaration.from_source(source, parent=parent)
