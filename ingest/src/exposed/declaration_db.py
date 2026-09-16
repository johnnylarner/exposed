"""Declaration SQL helpers accepting the legacy response-shaped model."""

from datetime import datetime
from uuid import UUID

from exposed.adapters.declaration_models import Declaration
from exposed.adapters.declaration_postgres import cohort as cohort
from exposed.adapters.declaration_postgres import write_declaration as write_parsed_declaration
from exposed.adapters.postgres import DatabaseConnection


def write_declaration(
    conn: DatabaseConnection,
    member_id: UUID,
    declaration: Declaration,
    fetched_at: datetime,
) -> None:
    write_parsed_declaration(conn, member_id, declaration.to_declaration(), fetched_at)
