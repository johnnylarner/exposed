"""Compatibility facade for the original declaration SQL helper signatures."""

from datetime import datetime
from uuid import UUID

from pydantic import JsonValue

from exposed.adapters.declaration_models import Declaration
from exposed.adapters.declaration_postgres import cohort as cohort
from exposed.adapters.declaration_postgres import write_declaration as write_snapshot
from exposed.adapters.postgres import DatabaseConnection
from exposed.core.declarations import DeclarationSnapshot


def write_declaration(
    conn: DatabaseConnection,
    member_id: UUID,
    declaration: Declaration,
    source: JsonValue,
    fetched_at: datetime,
) -> None:
    write_snapshot(
        conn,
        member_id,
        DeclarationSnapshot(
            declaration=declaration.to_declaration(),
            source_payload=source,
            fetched_at=fetched_at,
        ),
    )
