# Database migrations

[SQLx CLI 0.9.0](https://github.com/launchbadge/sqlx/tree/v0.9.0/sqlx-cli) applies
the single development baseline version, with an
[`up` migration](migrations/20260915000000_initial.up.sql) and a
[`down` migration](migrations/20260915000000_initial.down.sql).
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

The baseline uses [SQLx reversible migrations](https://github.com/launchbadge/sqlx/blob/v0.9.0/sqlx-cli/README.md#reverting-migrations).
Each direction runs in a transaction. `make db-revert` applies the down migration,
removing all five domain tables and their data, functions, `pg_trgm`, and the
`exposed` schema. It leaves SQLx's history table in `public`; `make db-migrate`
can then apply the baseline again. The down migration uses dependency order
without `CASCADE`, so unexpected dependencies cause a transactional failure.
`make db-nuke` deletes the entire configured database and recreates it from the
baseline.

The Makefile runs SQLx from `ingest/` to load `ingest/.env`, with environment
variables taking precedence. It explicitly selects the migration directory and
the root [`sqlx.toml`](../sqlx.toml), which fixes the history table at
`public._sqlx_migrations` regardless of the connection's search path. Direct SQLx
commands from the repository root read the root `.env` instead; set
`DATABASE_URL` explicitly when targeting another database. No migration command
runs automatically during an import.

Run only one import or migration against a database at a time.

## Timestamps

Every domain table has `created_at` and `updated_at`, both
`TIMESTAMPTZ NOT NULL DEFAULT now()`. A shared `BEFORE UPDATE` trigger function
sets `updated_at` when `OLD.* IS DISTINCT FROM NEW.*`; no-op updates and unchanged
upserts retain their timestamps. Updates preserve `created_at` unless the caller
explicitly changes it. PostgreSQL's `now()` is the transaction start time, so
changes within the same transaction share a timestamp.

All source date/time columns also use `TIMESTAMPTZ`. The importer converts
date-only values to midnight UTC at the persistence boundary, keeping calendar
dates in the domain model. It supplies timezone-aware parameters and reads stored
term starts in UTC, independent of the database session's timezone. PostgreSQL
stores instants; their displayed offset depends on the session timezone.
`members.latest_membership_from` remains text: it is a constituency or membership
description, not a date.

SQL uses uppercase keywords/types, lowercase snake_case identifiers, and
schema-qualified domain objects.

## Adopt an existing dbmate database

SQLx does not read `public.schema_migrations`. Running the baseline normally
against an existing schema would try to create tables that already exist.

1. Inspect the existing schema and compare it with a fresh database initialized
   from the baseline. Verify columns, nullability, constraints, indexes, the
   service-start and audit triggers, and the `pg_trgm` extension's schema. Old
   dbmate history alone is insufficient because the baseline has been edited during development.
2. Apply any missing schema changes in place, preserving imported data. This
   baseline includes `pg_trgm`, the later member/funding `NOT NULL` changes, and
   the timestamps described above.
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

Fold schema changes into the up/down pair without adding migration versions,
as required by [AGENTS.md](../AGENTS.md).
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
   baseline = Path('db/migrations/20260915000000_initial.up.sql')
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

### Upgrade the previous date-based baseline in place

For a database with the previous `DATE` columns and no audit columns, apply these
changes in a transaction. The explicit UTC conversion preserves calendar dates
regardless of the session timezone. Existing records receive the upgrade's
transaction timestamp for both audit columns; historical creation/update times
were not recorded and cannot be reconstructed.

```sql
BEGIN;

ALTER TABLE exposed.parliament_terms
    ALTER COLUMN term_start TYPE TIMESTAMPTZ
        USING term_start::TIMESTAMP AT TIME ZONE 'UTC',
    ALTER COLUMN term_end TYPE TIMESTAMPTZ
        USING term_end::TIMESTAMP AT TIME ZONE 'UTC',
    ADD COLUMN created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT now();

ALTER TABLE exposed.members
    ADD COLUMN created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT now();

ALTER TABLE exposed.member_terms
    ALTER COLUMN source_start_date TYPE TIMESTAMPTZ
        USING source_start_date::TIMESTAMP AT TIME ZONE 'UTC',
    ALTER COLUMN source_end_date TYPE TIMESTAMPTZ
        USING source_end_date::TIMESTAMP AT TIME ZONE 'UTC',
    ALTER COLUMN served_from TYPE TIMESTAMPTZ
        USING served_from::TIMESTAMP AT TIME ZONE 'UTC',
    ALTER COLUMN served_until TYPE TIMESTAMPTZ
        USING served_until::TIMESTAMP AT TIME ZONE 'UTC',
    ADD COLUMN created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT now();

ALTER TABLE exposed.declarations
    ALTER COLUMN registration_date TYPE TIMESTAMPTZ
        USING registration_date::TIMESTAMP AT TIME ZONE 'UTC',
    ADD COLUMN created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT now();

ALTER TABLE exposed.funding_entries
    ADD COLUMN created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT now();
```

Before committing, run the `CREATE FUNCTION exposed.set_updated_at()` statement
and all five audit `CREATE TRIGGER` statements from the up migration. Also apply
its `COMMENT ON COLUMN` statements and replace `check_term_service_start()` with
the baseline definition using `CREATE OR REPLACE FUNCTION`. Then `COMMIT`.

Compare the schema with a fresh database, including defaults, constraints,
indexes, functions, and triggers, and verify every original record, treating
midnight UTC timestamps as their original calendar dates. Only then update the
checksum using the procedure above. The migration version stays `20260915000000`;
SQLx records the checksum of the `.up.sql` file. Renaming the old simple migration
to this reversible pair does not require deleting its history row.

## Tests

`make check` uses the actual SQLx CLI to initialize isolated PostgreSQL databases
from the baseline for importer tests. Migration tests cover the consolidated
schema, audit defaults and change triggers, timezone-independent imports,
up/down/up cycles, repeat runs with data present, checksum mismatch detection,
and adopting an existing schema without losing records. They never reset the
application database.
