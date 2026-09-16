# Database migrations

[dbmate](https://github.com/amacneil/dbmate) owns schema migrations. SQL files live in `db/migrations/`; application code no longer applies migrations.

From the repository root:

```sh
make migration-new name=add_example
# Edit the generated SQL.
make migrate
make migration-status
```

Each file has `-- migrate:up` and `-- migrate:down` sections. dbmate applies each pending migration in its own transaction and records its numeric version in `public.schema_migrations`. Make corrections with new migrations once a migration has been shared. dbmate records version numbers, not content checksums.

The initial migration creates the current development schema, including nullable `term_end`. There is no upgrade path from the retired Python migration runner.

The Makefile explicitly selects the environment file, migration directory and history table. It disables automatic schema dumps so applying migrations does not require a local `pg_dump` installation. No migration command runs automatically during an import.

Run only one import or migration against a database at a time.

## Tests

`make check` uses the actual dbmate CLI to initialize isolated PostgreSQL databases for importer tests. Migration-specific tests are not retained. Reversing the initial migration drops its domain tables and their data.

The registration-date migration adds a nullable column without rewriting existing declarations.
After `make db-migrate`, use `./ingest/scripts/backfill-declaration-dates.sh` to populate it without
rerunning the funding import; see the [backfill instructions](../ingest/README.md#backfill-registration-dates-without-reimporting-funding).
