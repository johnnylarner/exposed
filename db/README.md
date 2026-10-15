# Database migrations

[SQLx CLI 0.9.0](https://github.com/launchbadge/sqlx/tree/v0.9.0/sqlx-cli) applies
versioned reversible migrations from `db/migrations`. The application exposes
`exposed migrate run` as an explicit migration command. Server startup and data
commands do not apply migrations. Run these commands from the repository root:

```sh
sqlx migrate run --config sqlx.toml --source db/migrations
sqlx migrate info --config sqlx.toml --source db/migrations
```

To apply migrations with the application binary:

```sh
exposed migrate run
exposed migrate run --source /srv/exposed/migrations
```

The default source is `db/migrations`, relative to the current working directory.
The binary reads migration files at runtime and applies pending up migrations.
It returns `{"source":"db/migrations"}` on stdout after success, or the existing
JSON error envelope on stderr with exit code 1 after failure. The response's
`source` is the supplied path. Use SQLx CLI for adding, inspecting, or reverting
migrations. The wrapper does not read `sqlx.toml`; it fixes the same history table
at `public._sqlx_migrations` and retains SQLx's locking and checksum checks.

Commands read the root `.env`. Set `DATABASE_URL` explicitly to target another
database. The root [`sqlx.toml`](../sqlx.toml) fixes the history table at
`public._sqlx_migrations`, regardless of the connection's search path.
Run only one import or migration against a database at a time.

## Add or revert a migration

Create a versioned pair with SQLx:

```sh
sqlx migrate add --source db/migrations --reversible describe_the_change
```

Write the schema change in the generated `.up.sql` file and its reverse in the
matching `.down.sql` file. Keep applied migrations immutable. Add new versions
for later changes instead of editing the baseline or creating `db/upgrades`
scripts. SQLx runs each direction in a transaction; omit `BEGIN` and `COMMIT`.

To revert the latest applied version:

```sh
sqlx migrate revert --config sqlx.toml --source db/migrations
```

Each invocation reverts one version. The current chain creates the normalized
member, declaration, funder, and funding tables, then adds the resolved declaration
loader, then adds the search indexes. Revert these in reverse order. Reverting
the loader drops its aliases, load history, and attribution columns. It also
restores unique funder names, so duplicate names cause the whole revert to fail
without changing the schema. Reverting the baseline removes all domain data,
`pg_trgm`, and the `exposed` schema. Unexpected dependencies fail transactionally
because the down migrations do not use `CASCADE`.

Test both a populated forward upgrade and the complete down/up cycle in an
isolated database before applying a new migration to imported data.

## Transition from the consolidated baseline once

Earlier development versions folded the loader and search changes into
`20260915000000_initial.up.sql`. The migration chain now assigns those changes
their own versions. An existing database with the consolidated baseline already
has the latest schema, but SQLx rejects the changed baseline checksum.

1. Back up the existing database and inspect `public._sqlx_migrations`.
   This procedure applies only when its sole successful migration is
   `20260915000000` and its schema already matches the consolidated baseline.
   If the schema or history differs, investigate those differences first.
2. Create an isolated database and run all current migrations there. Compare
   its schema with the existing database, including columns, nullability,
   defaults, constraints, indexes, functions, triggers, and the extension schema.
   Column order may differ. Verify that the existing database already contains
   every loader and search change. Save a record of the imported data for
   comparison after the transition.
3. Only after the schemas match, align the existing baseline checksum. From the
   repository root, with `DATABASE_URL` targeting the verified database:

   ```sh
   ingest/.venv/bin/python - <<'PY'
   import hashlib
   import os
   from pathlib import Path

   import psycopg
   from dotenv import load_dotenv

   load_dotenv('.env')
   checksum = hashlib.sha384(
       Path('db/migrations/20260915000000_initial.up.sql').read_bytes()
   ).digest()
   with psycopg.connect(os.environ['DATABASE_URL']) as conn:
       conn.execute('LOCK TABLE public._sqlx_migrations IN EXCLUSIVE MODE')
       history = conn.execute(
           'SELECT version, success FROM public._sqlx_migrations ORDER BY version'
       ).fetchall()
       if history != [(20260915000000, True)]:
           raise RuntimeError('Expected only the successful consolidated baseline')
       conn.execute(
           'UPDATE public._sqlx_migrations SET checksum = %s WHERE version = %s',
           (checksum, 20260915000000),
       )
   PY
   sqlx migrate run --config sqlx.toml --source db/migrations
   sqlx migrate info --config sqlx.toml --source db/migrations
   ```

4. Verify that all three versions succeeded, the schema still matches the
   isolated database, and imported records are unchanged. The loader and search
   up migrations tolerate their already-present objects and let SQLx record
   their versions normally. Do not use `migrate override skip` on the migration
   directory: it can mark unrelated pending migrations as applied.

This checksum alignment is a one-time transition, not the workflow for future
schema changes. For older dbmate databases or partially upgraded schemas,
compare and reconcile their actual schema and history separately before using
this chain. A history record alone does not prove schema compatibility.

## Timestamps

Each entity table has `created_at` and `updated_at`, both
`TIMESTAMPTZ NOT NULL DEFAULT now()`. A shared `BEFORE UPDATE` trigger function
sets `updated_at` when `OLD.* IS DISTINCT FROM NEW.*`; no-op updates and unchanged
upserts retain their timestamps. Updates preserve `created_at` unless the caller
explicitly changes it. PostgreSQL's `now()` is the transaction start time, so
changes within the same transaction share a timestamp.

`declaration_load_runs` records the first successful load time in `loaded_at`. Its
fingerprint and row counts remain fixed for that ingestion key.

All source date/time columns also use `TIMESTAMPTZ`. The importer converts
date-only values to midnight UTC at the persistence boundary, keeping calendar
dates in the domain model. It supplies timezone-aware parameters and reads stored
date values in UTC, independent of the database session's timezone. PostgreSQL
stores instants; their displayed offset depends on the session timezone.
`members.latest_membership_from` remains text: it is a constituency or membership
description, not a date.

New declaration ingestions populate registration dates when supplied by Parliament.
Missing source dates remain null.

SQL uses uppercase keywords/types, lowercase snake_case identifiers, and
schema-qualified domain objects.

## Deploy stored funder groups

The typed `ReviewedFamily` constants own the grouping policy in
`exposed/src/lib/domain/models/funder_grouping.rs`. Search and detail queries
read stored membership, direct evidence, and reviewed preferred names. Each family
sets `preferred_name` in the policy. Sparse `funder_group_names` rows keep these
display names separate from the minimum-UUID group identity and recorded names.
Schema migrations create empty projection tables; they do not apply the policy
to existing records.

Set `DATABASE_URL` to the target database. Before serving requests after the
grouping or preferred-name migration, stop the API, apply migrations, then run
the new binary against the same database. Use an ingestion key already loaded
into that database, with its unchanged raw, cleaned, and resolved artifacts. Set
`ingestion_key` to that key and use a YAML config whose `data_dir` contains the run.
`data latest --config exposed/config/cli-dev.yaml` shows the latest local
run, but confirm that the target database has already loaded it before proceeding.

Retry the load with those exact artifacts:

```sh
sqlx migrate run --config sqlx.toml --source db/migrations
cargo run --package exposed --bin exposed -- data declarations load --config exposed/config/cli-dev.yaml --ingestion-key "$ingestion_key"
psql "$DATABASE_URL" -X -v ON_ERROR_STOP=1 -c '
SELECT count(*) AS missing_memberships
FROM exposed.funders f
LEFT JOIN exposed.funder_alias_groups g ON g.funder_id = f.id
WHERE g.funder_id IS NULL;'
```

Require the load to report `Run already loaded` and `missing_memberships = 0`
before restarting the API. An exact retry refreshes all persisted funder groups,
direct evidence, and preferred names in one transaction under the declaration
loader lock. It preserves imported source data. Changed artifacts for an already
loaded key are rejected. Loading a new key can replace source data and prune
unreferenced funders and aliases, so use an exact retry for this procedure.

Repeat the exact retry after a reviewed policy update or manual changes to funder
names or aliases. Successful new declaration loads also refresh the projection in
their transaction. Keep the original artifacts available for future retries.

The alias evidence table uses a generated ID so long recorded names can be stored
without exceeding an index key limit.
