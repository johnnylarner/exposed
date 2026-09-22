"""Read the existing member cohort and persist accepted declarations."""

from collections.abc import Iterator
from contextlib import contextmanager
from datetime import date, datetime
from uuid import UUID, uuid7

import psycopg

from exposed.adapters.postgres import DatabaseConnection, date_timestamp, storage_error
from exposed.core.declaration_ports import DeclarationWriter
from exposed.core.declarations import Declaration, FundingEntry


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
            (date_timestamp(term_start),),
        )
    }


def write_declaration(
    conn: DatabaseConnection,
    member_id: UUID,
    declaration: Declaration,
    fetched_at: datetime,
) -> None:
    conn.execute(
        """INSERT INTO exposed.declarations (
               id, source_declaration_id, member_id, category_id, category_name,
               registration_date, fetched_at
           ) VALUES (%s, %s, %s, %s, %s, %s, %s)
           ON CONFLICT (source_declaration_id) DO UPDATE SET
               member_id = EXCLUDED.member_id, category_id = EXCLUDED.category_id,
               category_name = EXCLUDED.category_name,
               registration_date = EXCLUDED.registration_date,
               fetched_at = EXCLUDED.fetched_at""",
        (
            uuid7(),
            declaration.id,
            member_id,
            declaration.category_id,
            declaration.category_name,
            date_timestamp(declaration.registration_date),
            fetched_at,
        ),
    )

    previous = conn.execute(
        """SELECT funder, amount, currency, payment_type, donor_status, company_number
           FROM exposed.funding_entries
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
                   id, source_declaration_id, funder, amount, currency, payment_type,
                   donor_status, company_number
               ) VALUES (%s, %s, %s, %s, %s, %s, %s, %s)""",
            [
                (
                    uuid7(),
                    declaration.id,
                    entry.funder,
                    entry.amount,
                    entry.currency,
                    entry.payment_type,
                    entry.donor_status,
                    entry.company_number,
                )
                for entry in declaration.funding
            ],
        )


class _PostgresDeclarationWriter:
    def __init__(self, conn: DatabaseConnection):
        self.conn = conn

    def write_declaration(
        self, member_id: UUID, declaration: Declaration, fetched_at: datetime
    ) -> None:
        write_declaration(self.conn, member_id, declaration, fetched_at)


class PostgresDeclarationStore:
    """Own one transaction for each member's accepted declaration writes."""

    def __init__(self, conn: DatabaseConnection):
        self.conn = conn

    def cohort(self, term_start: date) -> dict[int, UUID]:
        try:
            return cohort(self.conn, term_start)
        except psycopg.Error as exc:
            raise storage_error(exc) from exc

    @contextmanager
    def refresh_member(self) -> Iterator[DeclarationWriter]:
        try:
            with self.conn.transaction():
                yield _PostgresDeclarationWriter(self.conn)
        except psycopg.Error as exc:
            raise storage_error(exc) from exc
