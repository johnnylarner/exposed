"""Read the existing member cohort and persist accepted declarations."""

from collections import Counter
from datetime import date, datetime
from uuid import UUID, uuid7

from psycopg.types.json import Jsonb
from pydantic import JsonValue

from exposed.db import DatabaseConnection
from exposed.declaration_models import Declaration, FundingEntry


def cohort(conn: DatabaseConnection, term_start: date) -> dict[int, UUID]:
    return {
        row["parliament_member_id"]: row["id"]
        for row in conn.execute(
            """SELECT DISTINCT m.id, m.parliament_member_id
               FROM exposed.members m
               JOIN exposed.member_terms s ON s.member_id = m.id
               JOIN exposed.parliament_terms t ON t.id = s.term_id
               WHERE t.term_start = %s AND s.house = 1
               ORDER BY m.parliament_member_id""",
            (term_start,),
        )
    }


def write_declaration(
    conn: DatabaseConnection,
    member_id: UUID,
    declaration: Declaration,
    source: JsonValue,
    fetched_at: datetime,
) -> None:
    conn.execute(
        """INSERT INTO exposed.declarations (
               id, source_declaration_id, member_id, category_id, category_name,
               source_payload, fetched_at
           ) VALUES (%s, %s, %s, %s, %s, %s, %s)
           ON CONFLICT (source_declaration_id) DO UPDATE SET
               member_id = EXCLUDED.member_id, category_id = EXCLUDED.category_id,
               category_name = EXCLUDED.category_name,
               source_payload = EXCLUDED.source_payload, fetched_at = EXCLUDED.fetched_at""",
        (
            uuid7(),
            declaration.id,
            member_id,
            declaration.category.id,
            declaration.category.name,
            Jsonb(source),
            fetched_at,
        ),
    )

    previous = conn.execute(
        """SELECT funder, amount, currency, payment_type FROM exposed.funding_entries
           WHERE source_declaration_id = %s""",
        (declaration.id,),
    ).fetchall()
    if Counter(FundingEntry.model_validate(row) for row in previous) == Counter(
        declaration.funding
    ):
        return
    conn.execute(
        "DELETE FROM exposed.funding_entries WHERE source_declaration_id = %s", (declaration.id,)
    )
    with conn.cursor() as cursor:
        cursor.executemany(
            """INSERT INTO exposed.funding_entries (
                   id, source_declaration_id, funder, amount, currency, payment_type
               ) VALUES (%s, %s, %s, %s, %s, %s)""",
            [
                (
                    uuid7(),
                    declaration.id,
                    entry.funder,
                    entry.amount,
                    entry.currency,
                    entry.payment_type,
                )
                for entry in declaration.funding
            ],
        )
