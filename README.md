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

## View example declarations from the API

These standalone scripts require `bash`, `curl` and `jq`; no database, API key or
Python setup is needed. Run from the repository root:

```sh
./scripts/view-individual-declaration.sh
./scripts/view-legal-entity-declaration.sh
# View the complete API response, including all returned fields and versions:
./scripts/view-individual-declaration.sh --raw
./scripts/view-legal-entity-declaration.sh --raw
```

The scripts fetch fixed examples directly from Parliament on each run and print
formatted JSON. The default view selects the version with the latest register
publication date and includes the MP, donor, donor status, amount, currency,
company number and how the support was received. Amounts remain decimal strings.

Examples verified on 20 September 2026:

| Script | Declaration | Source donor status | Support |
| --- | --- | --- | --- |
| Individual | [16901](https://interests-api.parliament.uk/api/v2/Interests/16901), Alex Burghart | Individual | David Robert Meller, GBP 2,000 |
| Legal entity | [16863](https://interests-api.parliament.uk/api/v2/Interests/16863), Dr Simon Opher | Company | Labour Together Limited (09630980), GBP 5,000 |

Both examples record support linked to the MP but received by a party organisation,
rather than a direct personal payment. The legal-entity example uses the API's
explicit `Company` status. These are illustrative records, not searches for every
individual or legal entity. Upstream records can change; the default view fails
if the expected donor status changes, and `--raw` lets you inspect the response.

Contains Parliamentary information licensed under the
[Open Parliament Licence v3.0](https://www.parliament.uk/site-information/copyright-parliament/open-parliament-licence/).

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
