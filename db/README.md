# Database migrations

[SQLx CLI 0.9.0](https://github.com/launchbadge/sqlx/tree/v0.9.0/sqlx-cli) applies
the single development baseline,
[`20260915000000_initial.sql`](migrations/20260915000000_initial.sql).
It creates the member, service, declaration and funding tables, their current
constraints, and `pg_trgm` in the `exposed` schema. Application code does not
apply migrations.

For a fresh database, from the repository root:

```sh
make db-start
make db-migrate
make db-migration-status
make import-members
make import-declarations
```

The baseline is a SQLx simple migration: the entire file runs in a transaction.
It contains only forward SQL; there is no rollback script. `make db-nuke`
deletes the configured database, including all imported data, and recreates it
from the baseline.

The Makefile runs SQLx from `ingest/` to load `ingest/.env`, with environment
variables taking precedence. It explicitly selects the migration directory and
the root [`sqlx.toml`](../sqlx.toml), which fixes the history table at
`public._sqlx_migrations` regardless of the connection's search path. Direct SQLx
commands from the repository root read the root `.env` instead; set
`DATABASE_URL` explicitly when targeting another database. No migration command
runs automatically during an import.

Run only one import or migration against a database at a time.

## Adopt an existing dbmate database

SQLx does not read `public.schema_migrations`. Running the baseline normally
against an existing schema would try to create tables that already exist.

1. Inspect the existing schema and compare it with a fresh database initialized
   from the baseline. Verify columns, nullability, constraints, indexes, the
   service-start trigger, and the `pg_trgm` extension's schema. Old dbmate history
   alone is insufficient because the baseline has been edited during development.
2. Apply any missing schema changes in place, preserving imported data. This
   baseline includes `pg_trgm` and the later member/funding `NOT NULL` changes.
   Resolve missing values explicitly before adding those constraints; do not
   replace missing source data with invented values.
3. Once the schema matches, record the baseline without executing it using
   SQLx 0.9's `migrate override skip`:

   ```sh
   (cd ingest && sqlx migrate override skip --config ../sqlx.toml --source ../db/migrations)
   make db-migrate
   make db-migration-status
   ```

4. Verify that all domain data is unchanged and the baseline is installed.
   The old `public.schema_migrations` table can remain as historical metadata;
   SQLx does not use it. Use SQLx for all subsequent migrations.

`override skip` records the current checksum without checking the domain schema.
Use it only after the comparison and any required in-place changes above.

## Editing the development baseline

Fold schema changes into the baseline without creating new migration files or
changing its version, as required by [AGENTS.md](../AGENTS.md).
`make db-add-migration` explains this policy and exits without creating a file.

SQLx records a SHA-384 checksum and rejects edits to an already applied migration.
It does not apply those edits to the database. For an existing database:

1. Inspect the live schema and apply the required changes in place.
2. Compare the result with a fresh database migrated from the edited baseline,
   and verify that imported records were preserved.
3. Only after verification, update the recorded checksum. From the repository
   root, this uses the same environment file as the Makefile:

   ```sh
   ingest/.venv/bin/python - <<'PY'
   import hashlib
   import os
   from pathlib import Path

   import psycopg
   from dotenv import load_dotenv

   load_dotenv('ingest/.env')
   baseline = Path('db/migrations/20260915000000_initial.sql')
   checksum = hashlib.sha384(baseline.read_bytes()).digest()
   with psycopg.connect(os.environ['DATABASE_URL']) as conn:
       updated = conn.execute(
           'UPDATE public._sqlx_migrations SET checksum = %s '
           'WHERE version = %s AND success = true',
           (checksum, 20260915000000),
       )
       if updated.rowcount != 1:
           raise RuntimeError('Expected one successfully applied baseline')
   PY
   make db-migrate
   make db-migration-status
   ```

Changing the checksum alone does not change the schema. If imported data is
disposable, `make db-nuke` installs the edited baseline instead.

New declaration ingestions populate parsed fields, including registration dates
when supplied by Parliament. Missing source dates remain null.

## Tests

`make check` uses the actual SQLx CLI to initialize isolated PostgreSQL databases
from the baseline for importer tests. Migration tests cover the consolidated
schema, repeat runs with data present, checksum mismatch detection, and adopting
an existing schema without losing records. They never reset the application
database.
