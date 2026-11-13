#!/usr/bin/env python3
"""Verify canonical member migration preservation on a disposable populated clone.

The baseline is read-only. The candidate must start as an identical, pre-migration
clone. This script adds legacy membership fixtures there, runs up/down/up, then
removes those fixtures and leaves the candidate upgraded. Re-run against a fresh
clone, never production.
Requires psql and sqlx on PATH; uses only Python's standard library.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import uuid


MIGRATION_VERSION = 20261112123714
MIGRATION_NAME = f"{MIGRATION_VERSION}_lords_registered_interests"


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def sql(database, statement):
    result = subprocess.run(
        ["psql", "-X", "--dbname", database, "-Atq", "-v", "ON_ERROR_STOP=1", "-c", statement],
        env={**os.environ, "PGTZ": "UTC"},
        check=True,
        capture_output=True,
        text=True,
    )
    # splitlines() would split literal Unicode characters inside
    # JSON strings (including U+0085 and U+2028), corrupting otherwise valid rows.
    return result.stdout.removesuffix("\n").split("\n") if result.stdout else []


def identifier(value):
    return '"' + value.replace('"', '""') + '"'


def columns(database):
    rows = sql(database, """
        SELECT json_build_array(c.relname, a.attname)::text
        FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
        JOIN pg_attribute a ON a.attrelid=c.oid
        WHERE n.nspname='exposed' AND c.relkind='r'
          AND a.attnum>0 AND NOT a.attisdropped
        ORDER BY c.relname,a.attname
    """)
    tables = {}
    for row in rows:
        table, column = json.loads(row)
        tables.setdefault(table, []).append(column)
    return tables


def records(database, original_columns, upgraded=False):
    tables = {}
    for table, names in original_columns.items():
        query = (
            "SELECT (to_jsonb(m) || jsonb_build_object("
            "'latest_membership_from', m.membership_from, "
            "'latest_house', c.id, "
            "'is_current_commons', COALESCE(c.id = 1 AND "
            "m.membership_end_date IS NULL AND m.membership_status_provenance <> 'legacy_former', false)))::text "
            "FROM exposed.members m "
            "JOIN exposed.membership_kinds k ON k.id = m.membership_kind "
            "LEFT JOIN exposed.chambers c ON c.id = k.chamber_id"
            if table == "members" and upgraded
            else f"SELECT to_jsonb(t)::text FROM exposed.{identifier(table)} t"
        )
        # Keep NUMERIC values exact, even when their scale exceeds IEEE precision.
        rows = [json.loads(row, parse_float=str) for row in sql(database, query)]
        tables[table] = sorted(
            json.dumps({name: row[name] for name in names}, sort_keys=True)
            for row in rows
        )
    return tables


def same_records(expected, actual, stage):
    require(expected.keys() == actual.keys(), f"{stage}: table inventory differs")
    for table, rows in expected.items():
        if rows != actual[table]:
            before = {row.get("id", json.dumps(row, sort_keys=True)): row
                      for row in map(json.loads, rows)}
            after = {row.get("id", json.dumps(row, sort_keys=True)): row
                     for row in map(json.loads, actual[table])}
            changed = {
                key: sorted(name for name in before[key] if before[key][name] != after[key].get(name))
                for key in before.keys() & after.keys() if before[key] != after[key]
            }
            raise RuntimeError(
                f"{stage}: {table} differs; rows {len(rows)} -> {len(actual[table])}; "
                f"missing IDs {sorted(before.keys() - after.keys())}; "
                f"extra IDs {sorted(after.keys() - before.keys())}; changed fields {changed}"
            )


def schema(database):
    queries = {
        "columns": """
            SELECT c.relname,a.attname,format_type(a.atttypid,a.atttypmod),a.attnotnull,
                   coalesce(pg_get_expr(d.adbin,d.adrelid),''),
                   col_description(c.oid,a.attnum)
            FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
            JOIN pg_attribute a ON a.attrelid=c.oid
            LEFT JOIN pg_attrdef d ON d.adrelid=c.oid AND d.adnum=a.attnum
            WHERE n.nspname='exposed' AND c.relkind='r'
              AND a.attnum>0 AND NOT a.attisdropped
            ORDER BY c.relname,a.attname
        """,
        "constraints": """
            SELECT c.relname,con.conname,pg_get_constraintdef(con.oid,true)
            FROM pg_constraint con JOIN pg_class c ON c.oid=con.conrelid
            JOIN pg_namespace n ON n.oid=c.relnamespace
            WHERE n.nspname='exposed' ORDER BY c.relname,con.conname
        """,
        "indexes": """
            SELECT c.relname,pg_get_indexdef(i.indexrelid)
            FROM pg_index i JOIN pg_class c ON c.oid=i.indrelid
            JOIN pg_namespace n ON n.oid=c.relnamespace
            WHERE n.nspname='exposed' ORDER BY c.relname,pg_get_indexdef(i.indexrelid)
        """,
        "triggers": """
            SELECT c.relname,t.tgenabled,pg_get_triggerdef(t.oid,true)
            FROM pg_trigger t JOIN pg_class c ON c.oid=t.tgrelid
            JOIN pg_namespace n ON n.oid=c.relnamespace
            WHERE n.nspname='exposed' AND NOT t.tgisinternal ORDER BY c.relname,t.tgname
        """,
        "relations": """
            SELECT c.relname,c.relkind,c.relpersistence,c.relrowsecurity,
                   obj_description(c.oid,'pg_class')
            FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
            WHERE n.nspname='exposed' AND c.relkind IN ('r','v','m','S')
            ORDER BY c.relname
        """,
        "routines": """
            SELECT p.proname,pg_get_function_identity_arguments(p.oid),
                   pg_get_functiondef(p.oid)
            FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace
            WHERE n.nspname='exposed'
            ORDER BY p.proname,pg_get_function_identity_arguments(p.oid)
        """,
    }
    return {name: sql(database, f"SELECT row_to_json(s)::text FROM ({query}) s")
            for name, query in queries.items()}


def migration_fingerprints(repository):
    return {direction: hashlib.sha256(
        (repository / "db/migrations" / f"{MIGRATION_NAME}.{direction}.sql").read_bytes()
    ).hexdigest() for direction in ("up", "down")}


def migrate(database, direction, repository, fingerprints):
    require(migration_fingerprints(repository) == fingerprints,
            "Migration source changed during verification; start again on a fresh clone")
    subprocess.run(
        ["sqlx", "migrate", direction, "--config", str(repository / "sqlx.toml"),
         "--source", str(repository / "db/migrations")],
        env={**os.environ, "DATABASE_URL": database, "PGTZ": "UTC"},
        check=True,
    )


def migration_history(database):
    return sql(database, """
        SELECT json_build_array(version,description,success,encode(checksum,'hex'))::text
        FROM _sqlx_migrations ORDER BY version
    """)


LEGACY_LORDS_KINDS = {
    "life peer": "life_peer",
    "bishops": "bishop",
    "bishop": "bishop",
    "life peer (judicial)": "judicial_peer",
    "judicial peer": "judicial_peer",
    "hereditary": "hereditary_peer",
    "hereditary peer": "hereditary_peer",
    "hereds given lps": "hereditary_life_peer",
    "hereditary peer given life peerage": "hereditary_life_peer",
    "excepted hereditary": "excepted_hereditary_peer",
    "excepted hereditary peer": "excepted_hereditary_peer",
}


def add_legacy_fixtures(database):
    fixtures = []
    cases = [(1, "Migration verifier constituency", "elected_mp"),
             (2, "Life peer", "life_peer"),
             (2, "Bishops", "bishop"),
             (2, "Life Peer (judicial)", "judicial_peer"),
             (2, "Hereditary", "hereditary_peer"),
             (2, "Hereds given LPs", "hereditary_life_peer"),
             (2, "Excepted Hereditary", "excepted_hereditary_peer"),
             (2, "Historical unrecognised membership", "other_lords")]
    for house, description, kind in cases:
        fixture_id = str(uuid.uuid4())
        rows = sql(database, f"""
            INSERT INTO exposed.members
                (id,parliament_member_id,name,party_id,party_name,latest_house,
                 latest_membership_from,is_current_commons,created_at,updated_at)
            SELECT '{fixture_id}'::uuid, candidate_id, 'Migration verifier legacy membership',
                   0, 'Migration verifier', {house}, '{description}', false,
                   '2001-02-03T04:05:06.123456Z', '2002-03-04T05:06:07.654321Z'
            FROM generate_series(2147483647,2147483000,-1) candidate_id
            WHERE NOT EXISTS (SELECT 1 FROM exposed.members WHERE parliament_member_id=candidate_id)
            LIMIT 1 RETURNING id
        """)
        require(rows == [fixture_id], "Could not insert a disposable legacy membership fixture")
        fixtures.append({"id": fixture_id, "latestHouse": house,
                         "isCurrentCommons": False, "membershipFrom": description,
                         "expectedKind": kind, "financialRows": 0,
                         "originalTimestampsDistinct": True})
    return fixtures


def check_new_member_fields(database, original_columns, original_members):
    current = columns(database)
    required = {
        "membership_from", "membership_kind", "membership_start_date", "membership_end_date",
        "membership_status", "membership_status_provenance", "membership_status_active", "source_captured_at",
        "source_capture_key", "commons_imported_at",
    }
    require(required <= set(current.get("members", [])), "Canonical shared member fields are missing")
    require(not {"latest_house", "latest_membership_from", "is_current_commons"} & set(current["members"]),
            "Obsolete member columns remain")
    require(not {"member_profiles", "lords_categories", "lords_interests"} & current.keys(),
            "Obsolete profile/category/interest tables remain")
    require({"chambers", "membership_kinds", "simple_declarations"} <= current.keys(),
            "Required canonical lookup/declaration tables are missing")
    expected_tables = set(original_columns) | {
        "chambers", "membership_kinds", "lords_imports", "simple_declaration_registers", "simple_declarations",
    }
    require(set(current) == expected_tables, "Unexpected migration table inventory")
    members = {row["id"]: row for row in (
        json.loads(raw) for raw in sql(database, "SELECT to_jsonb(m)::text FROM exposed.members m")
    )}
    for original in map(json.loads, original_members):
        commons = original["latest_house"] == 1
        current = original["is_current_commons"]
        expected = {
            "membership_kind": "elected_mp" if commons else LEGACY_LORDS_KINDS.get(original["latest_membership_from"].strip().lower(), "other_lords"),
            "membership_start_date": None,
            "membership_end_date": None,
            "membership_status": (
                "Legacy Commons current classification" if current else "Legacy Commons former classification"
            ) if commons else None,
            "membership_status_provenance": ("legacy_current" if current else "legacy_former") if commons else "source",
            "membership_status_active": None,
            "source_captured_at": None,
            "source_capture_key": None,
            "commons_imported_at": original["created_at"],
        }
        actual = {field: members[original["id"]][field] for field in expected}
        require(expected == actual,
                f"Upgrade changed legacy lifecycle/unknown dates/clocks/import provenance for {original['id']}: " +
                ', '.join(field for field in expected if expected[field] != actual[field]))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline-url", required=True, help="Read-only populated main schema")
    parser.add_argument("--candidate-url", required=True, help="Disposable identical clone; up/down/up runs here")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    baseline_name = sql(args.baseline_url, "SELECT current_database()")
    candidate_name = sql(args.candidate_url, "SELECT current_database()")
    require(baseline_name != candidate_name, "Baseline and disposable candidate database names must differ")
    repository = Path(__file__).resolve().parents[1]
    fingerprints = migration_fingerprints(repository)
    original_columns = columns(args.baseline_url)
    require({"latest_house", "latest_membership_from", "is_current_commons"} <=
            set(original_columns.get("members", [])), "Baseline must have the original main member schema")
    history = migration_history(args.baseline_url)
    require(all(json.loads(row)[0] < MIGRATION_VERSION for row in history),
            "Baseline must precede the member migration")
    pending = {int(path.name.split('_', 1)[0]) for path in (repository / 'db/migrations').glob('*.up.sql')}
    pending -= {json.loads(row)[0] for row in history}
    require(pending == {MIGRATION_VERSION}, "Exactly the member migration must be pending")
    baseline = records(args.baseline_url, original_columns)
    before_schema = schema(args.baseline_url)
    same_records(baseline, records(args.candidate_url, original_columns), "Identical clone guard")
    require(before_schema == schema(args.candidate_url), "Candidate must start at the same main schema")
    require(history == migration_history(args.candidate_url), "Candidate migration history differs")
    fixtures = add_legacy_fixtures(args.candidate_url)
    expected = records(args.candidate_url, original_columns)
    for stage, direction in (("Upgrade", "run"), ("Downgrade", "revert"), ("Second upgrade", "run")):
        migrate(args.candidate_url, direction, repository, fingerprints)
        same_records(expected, records(args.candidate_url, original_columns, direction == "run"), stage)
        if direction == "run":
            check_new_member_fields(args.candidate_url, original_columns, expected["members"])
        else:
            restored = schema(args.candidate_url)
            require(before_schema == restored, "Downgrade changed schema definitions: " +
                    ', '.join(name for name in before_schema if before_schema[name] != restored[name]))
            require(history == migration_history(args.candidate_url), "Downgrade changed original migration history")
        print(f"{stage}: original rows and member semantics preserved", flush=True)
    require(migration_fingerprints(repository) == fingerprints, "Migration source changed during verification")
    for fixture in fixtures:
        removed = sql(args.candidate_url,
                      f"DELETE FROM exposed.members WHERE id = '{fixture['id']}'::uuid RETURNING id")
        require(removed == [fixture["id"]], "Could not remove a disposable fixture after verification")
        fixture["removedAfterProof"] = True
    same_records(baseline, records(args.candidate_url, original_columns, True), "Final upgraded baseline projection")
    same_records(baseline, records(args.baseline_url, original_columns), "Read-only baseline recheck")
    require(before_schema == schema(args.baseline_url), "Baseline schema changed during verification")
    result = {
        "populatedMainUpgrade": True,
        "originalValuesPreserved": True,
        "originalFinancialRowsPreserved": True,
        "legacyClassificationPreservedWithoutInventedDates": True,
        "legacySourceClocksRemainUnknown": True,
        "downSchemaColumnsConstraintsIndexesTriggersRelationsRoutinesRestored": True,
        "upDownUp": True,
        "finalCandidateMatchesOriginalBaselineProjection": True,
        "migrationSha256": fingerprints,
        "baselineDatabase": baseline_name[0],
        "candidateDatabase": candidate_name[0],
        "originalTableCounts": {name: len(rows) for name, rows in baseline.items()},
        "disposableFixtures": fixtures,
    }
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result))


if __name__ == "__main__":
    main()
