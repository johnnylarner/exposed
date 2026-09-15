"""Database operations; the importer owns the transaction spanning all batches."""

import hashlib
from dataclasses import astuple
from datetime import date
from importlib.resources import files
from typing import Literal
from uuid import UUID, uuid7

from psycopg import Connection
from psycopg.rows import DictRow

from exposed.models import ImportValidationError, Member, ServicePeriod

type DatabaseConnection = Connection[DictRow]


def migrate(conn: DatabaseConnection) -> list[str]:
    applied: list[str] = []
    with conn.transaction():
        conn.execute("CREATE SCHEMA IF NOT EXISTS exposed")
        conn.execute("""
            CREATE TABLE IF NOT EXISTS exposed.schema_migrations (
                version text PRIMARY KEY,
                sha256 text NOT NULL,
                applied_at timestamptz NOT NULL DEFAULT clock_timestamp()
            )
        """)
        recorded = {
            row["version"]: row["sha256"]
            for row in conn.execute("SELECT version, sha256 FROM exposed.schema_migrations")
        }
        migrations = sorted(files("exposed").joinpath("migrations").iterdir(), key=lambda f: f.name)
        available = {f.name for f in migrations if f.name.endswith(".sql")}
        if recorded.keys() - available:
            raise ImportValidationError("Database schema is newer than this importer")
        for migration in migrations:
            if migration.name not in available:
                continue
            sql = migration.read_text(encoding="utf-8")
            checksum = hashlib.sha256(sql.encode()).hexdigest()
            if migration.name in recorded:
                if recorded[migration.name] != checksum:
                    raise ImportValidationError(f"Applied migration changed: {migration.name}")
                continue
            # Packaged migration SQL is trusted code, sent as UTF-8 bytes.
            conn.execute(sql.encode("utf-8"))
            conn.execute(
                "INSERT INTO exposed.schema_migrations (version, sha256) VALUES (%s, %s)",
                (migration.name, checksum),
            )
            applied.append(migration.name)
    return applied


def ensure_term(conn: DatabaseConnection, term_start: date) -> UUID:
    dates = {
        row["term_start"] for row in conn.execute("SELECT term_start FROM exposed.parliament_terms")
    }
    if dates - {term_start}:
        raise ImportValidationError(
            "This database contains another term. This first version refreshes one configured "
            "Parliament; term rollover requires an explicit migration."
        )
    return next(
        conn.execute(
            """INSERT INTO exposed.parliament_terms (id, term_start) VALUES (%s, %s)
           ON CONFLICT (term_start) DO UPDATE SET term_start = EXCLUDED.term_start
           RETURNING id""",
            (uuid7(), term_start),
        )
    )["id"]


def write_member(
    conn: DatabaseConnection,
    term_id: UUID,
    member: Member,
    periods: list[ServicePeriod],
) -> Literal["inserted", "updated", "unchanged"]:
    previous = conn.execute(
        """SELECT parliament_member_id, name, party_id, party_name, latest_house,
                  latest_membership_from, latest_membership_from_id, is_current_commons
           FROM exposed.members WHERE parliament_member_id = %s""",
        (member.parliament_member_id,),
    ).fetchone()
    values = astuple(member)
    change = (
        "inserted"
        if previous is None
        else ("unchanged" if tuple(previous.values()) == values else "updated")
    )
    member_id = next(
        conn.execute(
            """INSERT INTO exposed.members (
               id, parliament_member_id, name, party_id, party_name, latest_house,
               latest_membership_from, latest_membership_from_id, is_current_commons
           ) VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s)
           ON CONFLICT (parliament_member_id) DO UPDATE SET
               name = EXCLUDED.name, party_id = EXCLUDED.party_id,
               party_name = EXCLUDED.party_name, latest_house = EXCLUDED.latest_house,
               latest_membership_from = EXCLUDED.latest_membership_from,
               latest_membership_from_id = EXCLUDED.latest_membership_from_id,
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
                    p.source_start_date,
                    p.source_end_date,
                    p.served_from,
                    p.served_until,
                )
                for p in periods
            ],
        )
    conn.execute(
        """DELETE FROM exposed.member_terms
           WHERE member_id = %s AND term_id = %s
             AND NOT (source_start_date = ANY(%s))""",
        (member_id, term_id, [p.source_start_date for p in periods]),
    )
    return change
