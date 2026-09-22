import hashlib
import subprocess
from datetime import date
from pathlib import Path
from urllib.parse import parse_qsl, urlencode, urlsplit, urlunsplit
from uuid import uuid4

import pytest

from exposed.importer import connect
from tests.conftest import ROOT, sqlx_migrate

pytestmark = pytest.mark.integration
BASELINE = ROOT / "db/migrations/20260915000000_initial.sql"


@pytest.mark.parametrize("search_path", ["public", "exposed,public"])
def test_baseline_creates_current_schema_and_preserves_data_on_repeat(
    empty_database_url: str, search_path: str
):
    parts = urlsplit(empty_database_url)
    query = dict(parse_qsl(parts.query))
    query["options"] = f"-csearch_path={search_path}"
    url = urlunsplit(parts._replace(query=urlencode(query)))
    sqlx_migrate(url)

    term_id = uuid4()
    with connect(url) as conn:
        tables = conn.execute(
            "SELECT tablename FROM pg_tables WHERE schemaname = 'exposed' ORDER BY tablename"
        ).fetchall()
        assert [row["tablename"] for row in tables] == [
            "declarations",
            "funding_entries",
            "member_terms",
            "members",
            "parliament_terms",
        ]
        required = conn.execute(
            """SELECT table_name, column_name FROM information_schema.columns
               WHERE table_schema = 'exposed' AND is_nullable = 'NO' AND (
                   (table_name = 'members' AND column_name IN (
                       'party_id', 'party_name', 'latest_membership_from')) OR
                   (table_name = 'funding_entries' AND column_name IN (
                       'funder', 'currency', 'payment_type'))
               )"""
        ).fetchall()
        assert len(required) == 6
        extension = conn.execute(
            "SELECT extnamespace::regnamespace::text AS schema FROM pg_extension "
            "WHERE extname = 'pg_trgm'"
        ).fetchone()
        assert extension == {"schema": "exposed"}
        assert conn.execute(
            "SELECT exposed.word_similarity('example', 'example') AS score"
        ).fetchone() == {"score": 1.0}
        conn.execute(
            "INSERT INTO exposed.parliament_terms (id, term_start) VALUES (%s, %s)",
            (term_id, date(2024, 7, 4)),
        )

    sqlx_migrate(url)
    assert "installed" in sqlx_migrate(url, "info").stdout
    with connect(url) as conn:
        assert conn.execute("SELECT id FROM exposed.parliament_terms").fetchall() == [
            {"id": term_id}
        ]
        assert conn.execute(
            "SELECT version, success, checksum FROM public._sqlx_migrations"
        ).fetchall() == [
            {
                "version": 20260915000000,
                "success": True,
                "checksum": hashlib.sha384(BASELINE.read_bytes()).digest(),
            }
        ]
        assert conn.execute(
            "SELECT to_regclass('public.schema_migrations') AS history"
        ).fetchone() == {"history": None}


def test_adopting_an_existing_schema_preserves_records(empty_database_url):
    term_id = uuid4()
    with connect(empty_database_url) as conn:
        # Simulate an already verified schema installed before SQLx adoption.
        conn.execute(BASELINE.read_bytes())
        conn.execute("CREATE TABLE public.schema_migrations (version varchar(255) PRIMARY KEY)")
        conn.execute("INSERT INTO public.schema_migrations VALUES ('20260915000000')")
        conn.execute(
            "INSERT INTO exposed.parliament_terms (id, term_start) VALUES (%s, %s)",
            (term_id, date(2024, 7, 4)),
        )

    sqlx_migrate(empty_database_url, "override", "skip")
    sqlx_migrate(empty_database_url)
    with connect(empty_database_url) as conn:
        assert conn.execute("SELECT id FROM exposed.parliament_terms").fetchall() == [
            {"id": term_id}
        ]
        assert conn.execute("SELECT version FROM public.schema_migrations").fetchall() == [
            {"version": "20260915000000"}
        ]
        assert conn.execute("SELECT version, success FROM public._sqlx_migrations").fetchall() == [
            {"version": 20260915000000, "success": True}
        ]


def test_an_edited_baseline_is_rejected_without_applying_sql(database_url, tmp_path: Path):
    changed = tmp_path / BASELINE.name
    changed.write_bytes(BASELINE.read_bytes() + b"\nDROP SCHEMA exposed CASCADE;\n")
    # Pass a separate source directory without modifying the repository baseline.
    with pytest.raises(subprocess.CalledProcessError) as error:
        sqlx_migrate(database_url, source=tmp_path)
    assert "was previously applied but has been modified" in (
        error.value.stdout + error.value.stderr
    )
    with connect(database_url) as conn:
        assert conn.execute(
            "SELECT to_regclass('exposed.members') IS NOT NULL AS members_exist"
        ).fetchone() == {"members_exist": True}
    sqlx_migrate(database_url)
