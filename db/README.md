# Database migrations

[dbmate](https://github.com/amacneil/dbmate) applies the single development baseline,
[`20260915000000_initial.sql`](migrations/20260915000000_initial.sql).
It creates the member, service, declaration and funding tables, including
`registration_date`, `donor_status` and `company_number`. Application code does not
apply migrations.

From the repository root:

```sh
make db-migrate
make import-members
make import-declarations
```

The baseline has `-- migrate:up` and `-- migrate:down` sections. dbmate applies it
in a transaction and records its numeric version in `public.schema_migrations`.

During development, fold schema changes into this baseline without creating new
migration files or changing its version, as required by [AGENTS.md](../AGENTS.md).

`make db-migrate` installs the current baseline on a fresh database. dbmate records
versions rather than content checksums, so it does not reapply an edited baseline
to an existing database. Inspect the existing schema and apply the necessary SQL
changes in place to preserve imported data, then verify the resulting schema.
Recreating a development database also installs the new baseline, but removes
its imported data.

New declaration ingestions populate the parsed fields, including registration
dates when supplied by Parliament. Missing source dates remain null.

The Makefile explicitly selects the environment file, migration directory and history table. It disables automatic schema dumps so applying migrations does not require a local `pg_dump` installation. No migration command runs automatically during an import.

Run only one import or migration against a database at a time.

## Tests

`make check` uses the actual dbmate CLI to initialize isolated PostgreSQL databases
from the baseline for importer tests. Reversing the baseline drops its domain
tables and their data.
