"""Database operations; the importer owns the transaction spanning all batches."""

from collections.abc import Sequence
from datetime import date
from typing import Literal
from uuid import UUID, uuid7

from psycopg import Connection
from psycopg.rows import DictRow

from exposed.models import ImportValidationError, Member, ServicePeriod

type DatabaseConnection = Connection[DictRow]


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
    periods: Sequence[ServicePeriod],
) -> Literal["inserted", "updated", "unchanged"]:
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
