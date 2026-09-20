# Database migrations

[dbmate](https://github.com/amacneil/dbmate) applies the development baseline,
[`20260915000000_initial.sql`](migrations/20260915000000_initial.sql), followed by
additive migrations. The baseline creates the member, service, declaration and
funding tables, including `registration_date`. The funder-identification migration
adds nullable `donor_status` and `company_number` columns without rebuilding tables
or deleting existing data. Application code does not apply migrations.

From the repository root:

```sh
make db-migrate
make import-members
make import-declarations
```

The baseline has `-- migrate:up` and `-- migrate:down` sections. dbmate applies it
in a transaction and records its numeric version in `public.schema_migrations`.

For an existing database using the current baseline, run `make db-migrate` to add
the funder-identification columns, then use the
[backfill script](../ingest/README.md#backfill-funder-identification). Existing
funding UUIDs and financial values are retained. Reversing this additive migration
drops only the two new columns and their constraint.

Earlier edits to the development baseline still require recreating databases that
predate those edits. dbmate records versions rather than content checksums, so
`make db-migrate` does not reapply an edited baseline. Future changes that need to
preserve populated databases should use a new migration.

New declaration ingestions populate the parsed fields, including registration
dates when supplied by Parliament. Missing source dates remain null.

The Makefile explicitly selects the environment file, migration directory and history table. It disables automatic schema dumps so applying migrations does not require a local `pg_dump` installation. No migration command runs automatically during an import.

Run only one import or migration against a database at a time.

## Tests

`make check` uses the actual dbmate CLI to initialize isolated PostgreSQL databases
from all migrations for importer tests. Reversing the baseline drops its domain
tables and their data.
