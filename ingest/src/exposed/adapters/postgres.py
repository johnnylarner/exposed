"""PostgreSQL implementation of atomic Commons member batches."""

from collections.abc import Iterator, Sequence
from contextlib import contextmanager
from datetime import UTC, date, datetime, time
from uuid import UUID, uuid7

import psycopg
from psycopg import Connection
from psycopg.rows import DictRow, dict_row

from exposed.core.errors import StorageError
from exposed.core.models import Member, ServicePeriod, validate_configured_term
from exposed.core.ports import MemberWriter, WriteResult

type DatabaseConnection = Connection[DictRow]


def date_timestamp(value: date | None) -> datetime | None:
    """Store source calendar dates at midnight UTC regardless of the session timezone."""
    return datetime.combine(value, time.min, UTC) if value is not None else None


def ensure_term(conn: DatabaseConnection, term_start: date) -> UUID:
    dates = {
        row["term_start"].astimezone(UTC).date()
        for row in conn.execute("SELECT term_start FROM exposed.parliament_terms")
    }
    validate_configured_term(term_start, dates)
    return next(
        conn.execute(
            """INSERT INTO exposed.parliament_terms (id, term_start) VALUES (%s, %s)
           ON CONFLICT (term_start) DO UPDATE SET term_start = EXCLUDED.term_start
           RETURNING id""",
            (uuid7(), date_timestamp(term_start)),
        )
    )["id"]


def write_member(
    conn: DatabaseConnection,
    term_id: UUID,
    member: Member,
    periods: Sequence[ServicePeriod],
) -> WriteResult:
    previous = conn.execute(
        """SELECT parliament_member_id, name, party_id, party_name, latest_house,
                  latest_membership_from, is_current_commons
           FROM exposed.members WHERE parliament_member_id = %s""",
        (member.parliament_member_id,),
    ).fetchone()
    values = (
        member.parliament_member_id,
        member.name,
        member.party_id,
        member.party_name,
        member.latest_house,
        member.latest_membership_from,
        member.is_current_commons,
    )
    change = (
        "inserted"
        if previous is None
        else ("unchanged" if tuple(previous.values()) == values else "updated")
    )
    member_id = next(
        conn.execute(
            """INSERT INTO exposed.members (
               id, parliament_member_id, name, party_id, party_name, latest_house,
               latest_membership_from, is_current_commons
           ) VALUES (%s, %s, %s, %s, %s, %s, %s, %s)
           ON CONFLICT (parliament_member_id) DO UPDATE SET
               name = EXCLUDED.name, party_id = EXCLUDED.party_id,
               party_name = EXCLUDED.party_name, latest_house = EXCLUDED.latest_house,
               latest_membership_from = EXCLUDED.latest_membership_from,
               is_current_commons = EXCLUDED.is_current_commons
           RETURNING id""",
            (uuid7(), *values),
        )
    )["id"]
    with conn.cursor() as cursor:
        cursor.executemany(
            """INSERT INTO exposed.member_terms (
                   id, member_id, term_id, house, source_start_date, source_end_date,
                   served_from, served_until
               ) VALUES (%s, %s, %s, %s, %s, %s, %s, %s)
               ON CONFLICT (member_id, term_id, house, source_start_date) DO UPDATE SET
                   source_end_date = EXCLUDED.source_end_date,
                   served_from = EXCLUDED.served_from, served_until = EXCLUDED.served_until""",
            [
                (
                    uuid7(),
                    member_id,
                    term_id,
                    p.house,
                    date_timestamp(p.source_start_date),
                    date_timestamp(p.source_end_date),
                    date_timestamp(p.served_from),
                    date_timestamp(p.served_until),
                )
                for p in periods
            ],
        )
    conn.execute(
        """DELETE FROM exposed.member_terms
           WHERE member_id = %s AND term_id = %s
             AND NOT (source_start_date = ANY(%s))""",
        (member_id, term_id, [date_timestamp(p.source_start_date) for p in periods]),
    )
    return change


def connect(database_url: str) -> DatabaseConnection:
    return psycopg.Connection[DictRow].connect(
        database_url,
        autocommit=True,
        row_factory=dict_row,
        connect_timeout=10,
        application_name="exposed",
    )


def storage_error(exc: psycopg.Error) -> StorageError:
    # DSNs and server details can contain credentials; retain them only in the cause.
    code = exc.sqlstate or "unknown"
    return StorageError(f"Database operation failed ({type(exc).__name__}, SQLSTATE {code})")


class _PostgresWriter:
    def __init__(self, conn: DatabaseConnection, term_id: UUID):
        self.conn = conn
        self.term_id = term_id

    def write_member(self, member: Member, periods: Sequence[ServicePeriod]) -> WriteResult:
        return write_member(self.conn, self.term_id, member, periods)


class PostgresStore:
    """Use a composition-owned connection; own one transaction per member batch."""

    def __init__(self, conn: DatabaseConnection):
        self.conn = conn

    @contextmanager
    def refresh_batch(self, term_start: date) -> Iterator[MemberWriter]:
        try:
            with self.conn.transaction():
                term_id = ensure_term(self.conn, term_start)
                yield _PostgresWriter(self.conn, term_id)
        except psycopg.Error as exc:
            raise storage_error(exc) from exc
