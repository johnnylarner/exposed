#!/usr/bin/env python3
"""Check the profile migration on a disposable clone of a populated main database."""

import argparse
import json
import os
from pathlib import Path
import subprocess


def sql(database, statement):
    result = subprocess.run(
        ["psql", "-X", "--dbname", database, "-Atq", "-v", "ON_ERROR_STOP=1", "-c", statement],
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout.removesuffix("\n").split("\n") if result.stdout else []


def identifier(value):
    return '"' + value.replace('"', '""') + '"'


def columns(database):
    rows = sql(database, """
        SELECT c.relname || '|' || a.attname
        FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
        JOIN pg_attribute a ON a.attrelid=c.oid
        WHERE n.nspname='exposed' AND c.relkind='r'
          AND a.attnum>0 AND NOT a.attisdropped
        ORDER BY c.relname,a.attname
    """)
    tables = {}
    for row in rows:
        table, column = row.split("|", 1)
        tables.setdefault(table, []).append(column)
    return tables


def records(database, original_columns, unified=False):
    tables = {}
    for table, names in original_columns.items():
        query = (
            "SELECT (to_jsonb(m)||to_jsonb(p))::text FROM exposed.members m "
            "JOIN exposed.member_profiles p ON p.member_id=m.id AND p.chamber='commons'"
            if table == "members" and unified
            else f"SELECT to_jsonb(t)::text FROM exposed.{identifier(table)} t"
        )
        rows = [json.loads(row, parse_float=str) for row in sql(database, query)]
        tables[table] = sorted(
            json.dumps({name: row[name] for name in names}, sort_keys=True)
            for row in rows
        )
    return tables


def schema(database):
    queries = {
        "columns": """
            SELECT c.relname,a.attname,format_type(a.atttypid,a.atttypmod),a.attnotnull,
                   coalesce(pg_get_expr(d.adbin,d.adrelid),'')
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
            SELECT c.relname,pg_get_triggerdef(t.oid,true)
            FROM pg_trigger t JOIN pg_class c ON c.oid=t.tgrelid
            JOIN pg_namespace n ON n.oid=c.relnamespace
            WHERE n.nspname='exposed' AND NOT t.tgisinternal ORDER BY c.relname,t.tgname
        """,
    }
    return {name: sql(database, query) for name, query in queries.items()}


def migrate(database, direction, repository):
    subprocess.run(
        ["sqlx", "migrate", direction, "--config", str(repository / "sqlx.toml"),
         "--source", str(repository / "db/migrations")],
        env={**os.environ, "DATABASE_URL": database},
        check=True,
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline-url", required=True, help="Read-only populated main schema")
    parser.add_argument("--candidate-url", required=True, help="Disposable identical clone; up/down/up runs here")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.baseline_url == args.candidate_url:
        parser.error("baseline and disposable candidate must differ")
    repository = Path(__file__).resolve().parents[1]
    original_columns = columns(args.baseline_url)
    before = records(args.baseline_url, original_columns)
    before_schema = schema(args.baseline_url)
    assert before == records(args.candidate_url, original_columns), "Candidate must start as an identical clone"
    assert before_schema == schema(args.candidate_url), "Candidate must start at the same main schema"
    migrate(args.candidate_url, "run", repository)
    assert before == records(args.candidate_url, original_columns, True), "Upgrade changed existing values"
    migrate(args.candidate_url, "revert", repository)
    assert before == records(args.candidate_url, original_columns), "Downgrade changed existing values"
    assert before_schema == schema(args.candidate_url), "Downgrade changed schema definitions"
    migrate(args.candidate_url, "run", repository)
    assert before == records(args.candidate_url, original_columns, True), "Second upgrade changed existing values"
    result = {
        "populatedMainUpgrade": True,
        "originalValuesPreserved": True,
        "downSchemaColumnsConstraintsIndexesTriggersRestored": True,
        "upDownUp": True,
        "originalTableCounts": {name: len(rows) for name, rows in before.items()},
    }
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result))


if __name__ == "__main__":
    main()
