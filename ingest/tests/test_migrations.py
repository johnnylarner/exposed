import os
import subprocess
from pathlib import Path

import pytest

from exposed.importer import connect

pytestmark = pytest.mark.integration
ROOT = Path(__file__).resolve().parents[2]
BASELINE = 20260915000000
LOADER = 20261009000000
SEARCH = 20261009000001


def migrate(database_url, action="run", *extra, check=True):
    return subprocess.run(
        [
            os.environ.get("SQLX", "sqlx"),
            "migrate",
            action,
            "--config",
            str(ROOT / "sqlx.toml"),
            "--source",
            str(ROOT / "db/migrations"),
            "--database-url",
            database_url,
            *extra,
        ],
        check=check,
        capture_output=True,
        text=True,
    )


def versions(conn):
    return [
        row["version"]
        for row in conn.execute(
            "SELECT version FROM public._sqlx_migrations WHERE success ORDER BY version"
        ).fetchall()
    ]


def exists(conn, name):
    return conn.execute("SELECT to_regclass(%s) AS name", (name,)).fetchone()["name"] is not None


def test_migrations_revert_each_step_and_reapply(empty_database_url):
    migrate(empty_database_url)
    with connect(empty_database_url) as conn:
        assert versions(conn) == [BASELINE, LOADER, SEARCH]
        assert exists(conn, "exposed.funder_aliases_name_search_trigram_idx")

    migrate(empty_database_url, "revert")
    with connect(empty_database_url) as conn:
        assert versions(conn) == [BASELINE, LOADER]
        for table in ("members", "funders", "funder_aliases"):
            assert not exists(conn, f"exposed.{table}_name_search_trigram_idx")
        assert exists(conn, "exposed.funder_aliases")
        assert exists(conn, "exposed.declaration_load_runs")

    migrate(empty_database_url, "revert")
    with connect(empty_database_url) as conn:
        assert versions(conn) == [BASELINE]
        assert not exists(conn, "exposed.funder_aliases")
        assert not exists(conn, "exposed.declaration_load_runs")
        assert exists(conn, "exposed.funders_funder_name_key")
        assert not exists(conn, "exposed.funders_resolution_identity_id_key")
        assert not exists(conn, "exposed.funding_entries_source_funding_entry_id_key")

    migrate(empty_database_url, "revert")
    with connect(empty_database_url) as conn:
        assert versions(conn) == []
        assert (
            conn.execute("SELECT 1 FROM pg_namespace WHERE nspname = 'exposed'").fetchone() is None
        )
        assert (
            conn.execute("SELECT 1 FROM pg_extension WHERE extname = 'pg_trgm'").fetchone() is None
        )

    migrate(empty_database_url)
    with connect(empty_database_url) as conn:
        assert versions(conn) == [BASELINE, LOADER, SEARCH]
        assert exists(conn, "exposed.funder_aliases_name_search_trigram_idx")


def test_populated_baseline_upgrades_without_changing_records(empty_database_url):
    migrate(empty_database_url, "run", "--target-version", str(BASELINE))
    with connect(empty_database_url) as conn:
        conn.execute(
            "INSERT INTO exposed.members (parliament_member_id, name, party_id, party_name, "
            "latest_house, latest_membership_from, is_current_commons) "
            "VALUES (1, 'Member', 1, 'Party', 1, 'Constituency', true)"
        )
        conn.execute(
            "INSERT INTO exposed.declarations (source_declaration_id, member_id, "
            "category_id, category_name, fetched_at) "
            "SELECT 1, id, 1, 'Donations', now() FROM exposed.members"
        )
        conn.execute("INSERT INTO exposed.funders (funder_name) VALUES ('Donor')")
        conn.execute(
            "INSERT INTO exposed.funding_entries (source_declaration_id, funder_id, amount) "
            "SELECT 1, id, 100 FROM exposed.funders"
        )
        before = {
            table: conn.execute(f"SELECT * FROM exposed.{table}").fetchall()
            for table in ("members", "declarations", "funders", "funding_entries")
        }

    for _ in range(2):
        migrate(empty_database_url)
        with connect(empty_database_url) as conn:
            assert versions(conn) == [BASELINE, LOADER, SEARCH]
            for table, rows in before.items():
                after = conn.execute(f"SELECT * FROM exposed.{table}").fetchall()
                assert [{key: row[key] for key in rows[0]} for row in after] == rows
            funder = conn.execute("SELECT * FROM exposed.funders").fetchone()
            funding = conn.execute("SELECT * FROM exposed.funding_entries").fetchone()
            assert funder["resolution_identity_id"] is None
            assert funding["source_funding_entry_id"] is None
            assert funding["attribution_issues"] == []


def test_loader_revert_rolls_back_when_funder_names_repeat(empty_database_url):
    migrate(empty_database_url, "run", "--target-version", str(LOADER))
    with connect(empty_database_url) as conn:
        conn.execute(
            "INSERT INTO exposed.funders (funder_name, resolution_identity_id) "
            "VALUES ('Donor', 'identity-1'), ('Donor', 'identity-2')"
        )
        conn.execute(
            "INSERT INTO exposed.funder_aliases (funder_id, funder_alias) "
            "SELECT id, 'Alias' FROM exposed.funders"
        )

    result = migrate(empty_database_url, "revert", check=False)
    assert result.returncode != 0
    assert "funders_funder_name_key" in result.stdout + result.stderr
    with connect(empty_database_url) as conn:
        assert versions(conn) == [BASELINE, LOADER]
        assert exists(conn, "exposed.declaration_load_runs")
        assert conn.execute("SELECT count(*) AS count FROM exposed.funder_aliases").fetchone() == {
            "count": 2
        }
        assert conn.execute(
            "SELECT resolution_identity_id FROM exposed.funders ORDER BY resolution_identity_id"
        ).fetchall() == [
            {"resolution_identity_id": "identity-1"},
            {"resolution_identity_id": "identity-2"},
        ]
        assert conn.execute(
            "SELECT count(attribution_issues) AS count FROM exposed.funding_entries"
        ).fetchone() == {"count": 0}
