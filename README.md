# exposed

Where do MPs get their money from?

The first milestone imports everyone who has served in the Commons during the configured Parliament into PostgreSQL.

## Getting started

Requires Python **3.14+**, Docker Compose and [dbmate](https://github.com/amacneil/dbmate) (verified with 2.35.1). On macOS, install dbmate with `brew install dbmate`.

Run commands from the repository root:

```sh
make db-start
make migrate
make import-members
```

Setup creates `ingest/.env` only when absent. Both dbmate and the importer use that file; existing environment variables take precedence. The example URL targets local PostgreSQL and explicitly disables TLS for that loopback connection. Use the appropriate TLS setting for a hosted database.

See [migration instructions](db/README.md) for the SQL workflow.

## Useful commands

Run `make help` for all commands.

| Command | Purpose |
| --- | --- |
| `make migration-new name=add_example` | Create a timestamped SQL migration |
| `make migrate` | Apply pending migrations |
| `make migration-status` | Show applied and pending migrations |
| `make import-members` | Refresh member data |
| `make verify` | Check data in the local Compose database |
| `make check` | Run lint, formatting checks, type checks and all tests |
| `make test-unit` | Run tests without PostgreSQL |
| `make format` | Format Python files |
| `make db-stop` | Stop local PostgreSQL while retaining data |

`make test` and `make check` require local PostgreSQL (`make db-start`). Tests create and remove their own temporary databases. Override `EXPOSED_TEST_ADMIN_DSN` with a PostgreSQL URL to use a different test server; it must allow database creation. `DBMATE` and `PYTHON` can also be overridden.

See the [importer documentation](ingest/README.md) for configuration, data rules and refresh behavior.
