import os
import subprocess
from pathlib import Path
from typing import LiteralString, cast

import psycopg
import pytest
from psycopg import sql

from exposed.importer import connect
from tests.declaration_fakes import DeclarationsFixture, declaration
from tests.fakes import ParliamentFixture
from tests.test_declaration_importer import run
from tests.test_importer import dataset as member_dataset
from tests.test_importer import run as import_members

pytestmark = pytest.mark.integration
ROOT = Path(__file__).resolve().parents[2]
# The checked-in upgrade is trusted SQL, not an external query fragment.
UPGRADE = sql.SQL(cast(LiteralString, (ROOT / "db/upgrades/normalize_funders.sql").read_text()))


def schema(conn):
    return {
        "columns": conn.execute("""
            SELECT table_name, column_name, data_type, is_nullable, column_default
            FROM information_schema.columns WHERE table_schema = 'exposed'
            ORDER BY table_name, column_name
        """).fetchall(),
        "constraints": conn.execute("""
            SELECT conrelid::regclass::text AS relation, conname, pg_get_constraintdef(oid)
            FROM pg_constraint WHERE connamespace = 'exposed'::regnamespace
            ORDER BY relation, conname
        """).fetchall(),
        "indexes": conn.execute("""
            SELECT indexname, indexdef FROM pg_indexes WHERE schemaname = 'exposed'
            ORDER BY indexname
        """).fetchall(),
        "functions": conn.execute("""
            SELECT proname, pg_get_functiondef(oid) FROM pg_proc
            WHERE pronamespace = 'exposed'::regnamespace ORDER BY proname, oid::regprocedure::text
        """).fetchall(),
        "comments": conn.execute("""
            SELECT c.relname, a.attname, col_description(c.oid, a.attnum)
            FROM pg_class c JOIN pg_attribute a ON a.attrelid = c.oid
            WHERE c.relnamespace = 'exposed'::regnamespace AND a.attnum > 0 AND NOT a.attisdropped
            ORDER BY c.relname, a.attname
        """).fetchall(),
        "triggers": conn.execute("""
            SELECT tgname, pg_get_triggerdef(oid), tgenabled FROM pg_trigger
            WHERE NOT tgisinternal AND tgrelid IN (
                SELECT oid FROM pg_class WHERE relnamespace = 'exposed'::regnamespace
            ) ORDER BY tgname
        """).fetchall(),
    }


@pytest.fixture
def legacy_database(database_url):
    import_members(database_url, ParliamentFixture(1))
    run(database_url, DeclarationsFixture(declaration()))
    with connect(database_url) as conn:
        current_schema = schema(conn)
        conn.execute("""
            DROP TABLE exposed.funding_entries;
            DROP TABLE exposed.funders;
            CREATE TABLE exposed.funding_entries (
                id UUID PRIMARY KEY DEFAULT uuidv7(),
                source_declaration_id INTEGER NOT NULL
                    REFERENCES exposed.declarations (source_declaration_id),
                funder TEXT NOT NULL,
                amount NUMERIC,
                currency TEXT NOT NULL,
                payment_type TEXT NOT NULL,
                donor_status TEXT,
                company_number TEXT,
                created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                CONSTRAINT funding_entries_company_number_status CHECK (
                    company_number IS NULL OR donor_status IS NOT DISTINCT FROM 'Company'
                )
            );
            CREATE INDEX funding_entries_declaration_idx
                ON exposed.funding_entries (source_declaration_id);
            CREATE TRIGGER funding_entries_set_updated_at
                BEFORE UPDATE ON exposed.funding_entries FOR EACH ROW
                WHEN (OLD.* IS DISTINCT FROM NEW.*)
                EXECUTE FUNCTION exposed.set_updated_at();
            INSERT INTO exposed.funding_entries (
                source_declaration_id, funder, donor_status, company_number,
                amount, currency, payment_type
            ) VALUES
                (101, 'Company', 'Company', '00123456', 12.50, 'GBP', 'Monetary'),
                (101, 'Company', 'Company', '00123456', 12.50, 'GBP', 'Monetary'),
                (101, 'Person', 'Individual', NULL, 20, 'EUR', 'In kind');
        """)
    return database_url, current_schema


def test_upgrade_preserves_imported_data_and_matches_the_baseline(legacy_database):
    url, baseline = legacy_database
    members = member_dataset(url)
    with connect(url) as conn:
        payments = conn.execute("SELECT * FROM exposed.funding_entries ORDER BY id").fetchall()
        declarations = conn.execute("SELECT * FROM exposed.declarations ORDER BY id").fetchall()
        conn.execute(UPGRADE)
        assert schema(conn) == baseline
        assert (
            conn.execute("SELECT * FROM exposed.declarations ORDER BY id").fetchall()
            == declarations
        )
        assert (
            conn.execute("""
            SELECT fe.id, fe.source_declaration_id, f.funder_name AS funder,
                   fe.amount, fe.currency, fe.payment_type, f.funder_kind AS donor_status,
                   f.company_number, fe.created_at, fe.updated_at
            FROM exposed.funding_entries fe JOIN exposed.funders f ON f.id = fe.funder_id
            ORDER BY fe.id
        """).fetchall()
            == payments
        )
        identities = conn.execute("SELECT * FROM exposed.funders ORDER BY id").fetchall()
        assert len(identities) == 2
        conn.execute(UPGRADE)
        assert conn.execute("SELECT * FROM exposed.funders ORDER BY id").fetchall() == identities
        assert schema(conn) == baseline
    assert member_dataset(url) == members


def test_conflicting_legacy_metadata_aborts_without_losing_data(legacy_database):
    url, _ = legacy_database
    with connect(url) as conn:
        conn.execute("""
            UPDATE exposed.funding_entries SET company_number = '99999999'
            WHERE id = (SELECT id FROM exposed.funding_entries WHERE funder = 'Company' LIMIT 1)
        """)
        before = conn.execute("SELECT * FROM exposed.funding_entries ORDER BY id").fetchall()
        with pytest.raises(psycopg.errors.RaiseException, match="conflicting"):
            conn.execute(UPGRADE)
        conn.execute("ROLLBACK")
        assert (
            conn.execute("SELECT * FROM exposed.funding_entries ORDER BY id").fetchall() == before
        )
        assert conn.execute("SELECT to_regclass('exposed.funders') AS funders").fetchone() == {
            "funders": None
        }


def test_upgrade_already_normalized_feature_schema(database_url):
    with connect(database_url) as conn:
        baseline = schema(conn)
        conn.execute("""
            ALTER TABLE exposed.funding_entries
                ALTER COLUMN funder_id SET NOT NULL,
                ALTER COLUMN currency SET NOT NULL,
                ALTER COLUMN payment_type SET NOT NULL;
            DROP INDEX exposed.funding_entries_funder_idx;
        """)
        conn.execute(UPGRADE)
        assert schema(conn) == baseline


def test_single_baseline_reverts_and_reapplies(empty_database_url):
    args = [
        os.environ.get("SQLX", "sqlx"),
        "migrate",
    ]
    options = [
        "--config",
        str(ROOT / "sqlx.toml"),
        "--source",
        str(ROOT / "db/migrations"),
        "--database-url",
        empty_database_url,
    ]
    for action in ("run", "revert", "run"):
        subprocess.run([*args, action, *options], check=True, capture_output=True, text=True)
        with connect(empty_database_url) as conn:
            result = conn.execute("SELECT to_regclass('exposed.funders') AS funders").fetchone()
            assert result is not None
            assert (result["funders"] is None) == (action == "revert")
    with connect(empty_database_url) as conn:
        assert conn.execute("SELECT version FROM public._sqlx_migrations").fetchall() == [
            {"version": 20260915000000}
        ]
