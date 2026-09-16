"""Read the existing member cohort and persist accepted declarations."""

from collections.abc import Iterator
from contextlib import contextmanager
from datetime import date
from uuid import UUID, uuid7

import psycopg
from psycopg.types.json import Jsonb

from exposed.adapters.postgres import DatabaseConnection, storage_error
from exposed.core.declaration_ports import DeclarationWriter
from exposed.core.declarations import DeclarationSnapshot, FundingEntry


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
    snapshot: DeclarationSnapshot,
) -> None:
    declaration = snapshot.declaration
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
            declaration.category_id,
            declaration.category_name,
            Jsonb(snapshot.source_payload),
            snapshot.fetched_at,
        ),
    )

    previous = conn.execute(
        """SELECT funder, amount, currency, payment_type FROM exposed.funding_entries
           WHERE source_declaration_id = %s""",
        (declaration.id,),
    ).fetchall()
    if declaration.has_same_funding(tuple(FundingEntry.model_validate(row) for row in previous)):
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


class _PostgresDeclarationWriter:
    def __init__(self, conn: DatabaseConnection, term_start: date):
        self.conn, self.term_start = conn, term_start

    def cohort(self) -> dict[int, UUID]:
        return cohort(self.conn, self.term_start)

    def write_declaration(self, member_id: UUID, snapshot: DeclarationSnapshot) -> None:
        write_declaration(self.conn, member_id, snapshot)


class PostgresDeclarationStore:
    """Own one transaction for cohort reads and every accepted declaration write."""

    def __init__(self, conn: DatabaseConnection):
        self.conn = conn

    @contextmanager
    def refresh(self, term_start: date) -> Iterator[DeclarationWriter]:
        try:
            with self.conn.transaction():
                yield _PostgresDeclarationWriter(self.conn, term_start)
        except psycopg.Error as exc:
            raise storage_error(exc) from exc
