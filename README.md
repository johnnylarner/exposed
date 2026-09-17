# exposed

Where do MPs get their money from?

Imports everyone who has served in the Commons during the configured Parliament and their available declarations into PostgreSQL. Declarations store parsed fields and extracted funding for later display.

The planned journalist-facing research application is described in the
[backend product specification](docs/specs/journalist-research-backend.md).

## Getting started

Requires Python **3.14+**, Docker Compose and [dbmate](https://github.com/amacneil/dbmate) (verified with 2.35.1). On macOS, install dbmate with `brew install dbmate`.

Run commands from the repository root:

```sh
python3.14 -m venv ingest/.venv
ingest/.venv/bin/python -m pip install -r ingest/requirements.lock
ingest/.venv/bin/python -m pip install --no-deps -e ./ingest
test -f ingest/.env || cp ingest/.env.example ingest/.env
make db-start
make db-migrate
make import-members
make import-declarations
```

Setup creates `ingest/.env` only when absent. Both dbmate and the importer use that file; existing environment variables take precedence. The example URL targets local PostgreSQL and explicitly disables TLS for that loopback connection. Use the appropriate TLS setting for a hosted database.

The schema is a single development baseline. See [migration instructions](db/README.md)
for recreating development databases after schema changes and ingesting the current fields.

## Useful commands

| Command | Purpose |
| --- | --- |
| `make db-migrate` | Apply the development schema baseline |
| `make import-members` | Refresh member data |
| `make import-declarations` | Refresh declarations for the stored member cohort |
| `make verify` | Check data in the local Compose database |
| `make check` | Run lint, formatting checks, type checks and all tests |
| `make test-unit` | Run tests without PostgreSQL |
| `make format` | Format Python files |
| `make db-stop` | Stop local PostgreSQL while retaining data |

`make test` and `make check` require local PostgreSQL (`make db-start`). Tests create and remove their own temporary databases. Override `EXPOSED_TEST_ADMIN_DSN` with a PostgreSQL URL to use a different test server; it must allow database creation. `DBMATE` can also be overridden.

See the [importer documentation](ingest/README.md) for configuration, data rules and refresh behavior.
