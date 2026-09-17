# Database migrations

[dbmate](https://github.com/amacneil/dbmate) applies the single development baseline,
[`20260915000000_initial.sql`](migrations/20260915000000_initial.sql).
It creates the member, service, declaration, and funding tables, including
`registration_date`. Application code does not apply migrations.

From the repository root:

```sh
make db-migrate
make import-members
make import-declarations
```

The baseline has `-- migrate:up` and `-- migrate:down` sections. dbmate applies it
in a transaction and records its numeric version in `public.schema_migrations`.

During development, edit this baseline directly when the schema changes. Recreate
an existing development database and ingest members and declarations again to use
the updated schema. dbmate records versions rather than content checksums, so
`make db-migrate` does not reapply an edited baseline to an existing database.
Upgrade compatibility with earlier development schemas is not maintained.

New declaration ingestions populate the parsed fields, including registration
dates when supplied by Parliament. Missing source dates remain null.

The Makefile explicitly selects the environment file, migration directory and history table. It disables automatic schema dumps so applying migrations does not require a local `pg_dump` installation. No migration command runs automatically during an import.

Run only one import or migration against a database at a time.

## Tests

`make check` uses the actual dbmate CLI to initialize isolated PostgreSQL databases
from the baseline for importer tests. Reversing the baseline drops its domain
tables and their data.
