# Development database

SQLx CLI 0.9.0 applies one reversible baseline:
`migrations/20260915000000_initial.{up,down}.sql`. Application startup and imports
never run migrations. Domain tables are in `exposed`; SQLx history is in
`public._sqlx_migrations`, fixed by the root `sqlx.toml`.

For a new database, run from the repository root:

```sh
make db-start
make db-migrate
make db-migration-status
docker compose up --build -d exposed
make initialize
```

SQLx commands run from `ingest/` to load its explicit `.env`; environment variables
win. `DATABASE_URL` there is used by SQLx only. Python imports now talk to Rust's
operator API. Rust database configuration is in `exposed/config/`.

## Editing or adopting the baseline

Follow [AGENTS.md](../AGENTS.md): fold development changes into the single up/down
pair and preserve existing imported records. An edited checksum does not apply
schema changes. Stop imports before migrations; the application serializes imports
with a PostgreSQL advisory lock but migrations remain an operator responsibility.

For this ownership refactor, the only additions are `import_refresh_state` and
`import_notifications`. Existing member, service, declaration, funder and funding
columns are unchanged. The former stores the last completed source check, attempt,
retry time, outcome, rejection count and newly observed MPs. The latter stores
pending/delivered notification summaries and independent retry state. Neither is
an ingestion event archive or a delta-resolution system.

For an existing database:

1. Inspect the applied schema and SQLx migration history. Preserve a data snapshot
   or record-level fingerprints, including IDs and audit timestamps.
2. With `DATABASE_URL` exported, apply the additive upgrade with errors stopping
   execution:

   ```sh
   psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f db/upgrades/import_refresh_state.sql
   ```

3. Initialize a separate disposable database from the edited baseline. Compare
   columns (including defaults/nullability), constraints, indexes, functions,
   triggers and the `pg_trgm` extension/schema. Column order can differ after
   historic in-place upgrades. Verify all imported records against the snapshot.
   The upgrade's `IF NOT EXISTS` allows reruns but does not establish equivalence.
4. **Only after that verification**, update the recorded checksum. This example
   uses Python's standard library, the existing dotenv package, and `psql`; no
   Python database driver is needed:

   ```sh
   ingest/.venv/bin/python - <<'PY'
   import hashlib
   import os
   import subprocess
   from pathlib import Path
   from dotenv import load_dotenv

   load_dotenv('ingest/.env', override=False)
   checksum = hashlib.sha384(
       Path('db/migrations/20260915000000_initial.up.sql').read_bytes()
   ).hexdigest()
   statement = f"""
   DO $$
   DECLARE changed INTEGER;
   BEGIN
       UPDATE public._sqlx_migrations SET checksum = decode('{checksum}', 'hex')
       WHERE version = 20260915000000 AND success = true;
       GET DIAGNOSTICS changed = ROW_COUNT;
       IF changed <> 1 THEN
           RAISE EXCEPTION 'Expected one successfully applied baseline';
       END IF;
   END;
   $$;
   """
   subprocess.run(
       ['psql', os.environ['DATABASE_URL'], '--no-psqlrc', '-v', 'ON_ERROR_STOP=1'],
       input=statement, text=True, check=True,
   )
   PY
   make db-migrate
   make db-migration-status
   ```

If adopting a schema with no SQLx history, inspect and reconcile it first, then use
`sqlx migrate override skip --config ../sqlx.toml --source ../db/migrations` from
`ingest/` to record the verified baseline. Do not use this to conceal a mismatch.
Old dbmate history alone cannot establish that the schema matches the edited baseline.

`make db-revert` drops all eight application tables, functions, the extension and
schema; SQLx history remains. `make db-nuke` deletes the entire configured database.
Neither is part of normal initialization or recovery of an existing database.

## Data semantics

All domain tables have `created_at` and `updated_at`, with a shared trigger that
changes `updated_at` only when row values change. Date-only source values become
midnight UTC timestamps at the persistence boundary. Missing dates stay null.
An unchanged member upsert retains its timestamps. Declaration retrieval updates
`fetched_at`, so reparsing unchanged evidence is not literally a no-write operation.

Funders are shared by normalized name. Funding rows reference them through a
nullable FK. Metadata omissions retain known details; explicit corrections update
them, and non-company status clears the company number. Funding equality retains
multiplicity and ignores order; unchanged payments retain UUIDs. A changed group
is replaced atomically with its declaration. Missing upstream records are retained.

SQL uses uppercase keywords/types, lowercase snake_case identifiers and
schema-qualified application objects. `pg_trgm` is in the `exposed` schema. Existing
search query macros require `exposed,public` on the build connection's search path.

## Verification

`make test-rust` creates temporary PostgreSQL databases from the actual baseline.
It tests atomic writes, rollback, IDs/timestamps, shared metadata, repeat imports,
up/down/up, and applying the additive upgrade twice without losing domain data.
It never resets the application database. See
[the ownership migration verification](../docs/verification/rust-import-ownership.md)
for the inspected development database.
