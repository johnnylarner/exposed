# exposed

Where do MPs get their money from?

Imports everyone who has served in the Commons during the configured Parliament and their available declarations into PostgreSQL. Declarations store parsed fields and extracted funding for later display.

The planned journalist-facing research application is described in the
[backend product specification](docs/specs/journalist-research-backend.md).

## Getting started

Requires Python **3.14+**, Docker Compose, a Rust toolchain and
[SQLx CLI](https://github.com/launchbadge/sqlx/tree/v0.9.0/sqlx-cli) **0.9.0**
(matching the Rust workspace). Install the PostgreSQL CLI with:

```sh
cargo install sqlx-cli --version 0.9.0 --no-default-features --features rustls,postgres,sqlx-toml
```

Run commands from the repository root:

```sh
python3.14 -m venv ingest/.venv
ingest/.venv/bin/python -m pip install -r ingest/requirements.lock
ingest/.venv/bin/python -m pip install --no-deps -e ./ingest
test -f ingest/.env || cp ingest/.env.example ingest/.env
make db-start
make db-migrate
docker compose up --build -d exposed
make initialize
```

Setup creates `ingest/.env` only when absent. The Makefile runs SQLx from `ingest/`
so it and the operator command use that file; existing environment variables take
precedence. The example URL targets local PostgreSQL and explicitly disables TLS
for that loopback connection. Use the appropriate TLS setting for a hosted database.

The schema uses a single development baseline. See the
[migration instructions](db/README.md) before adopting SQLx on an existing dbmate
database or editing the baseline. See the
[import architecture](docs/architecture.md) for imported funding and ownership.

## Develop the Rust app with Docker Compose

Initialize the database using the setup above before starting the app. SQLx checks
queries against the live schema during compilation, so a fresh database needs
`make db-migrate` first. Migrations remain an explicit step.

```sh
docker compose up --build exposed
```

The search API listens at `http://localhost:6999`; the operator API is exposed on
loopback port 7000. Rust schedules declaration refreshes daily and retries failures.
Member refreshes remain manual (`make import-members`). The Compose development
token is `exposed_local_import`; set `EXPOSED_IMPORT_TOKEN` to override it in both
the container environment and the operator command. For example:

```sh
curl 'http://localhost:6999/search?term=McDonald'
```

The repository is bind-mounted into the container. `cargo watch` rebuilds and
restarts the app when Rust source, Cargo manifests, `Cargo.lock`, or files in
`exposed/config/` change. Polling detects edits through Docker Desktop bind mounts.
The first image and app builds take longer; named volumes cache Cargo downloads
and Linux build artifacts separately from the host's `target/` directory.

The container uses `exposed/config/compose.yaml` and connects to `postgres:5432`.
Its `DATABASE_URL` supplies the same connection for SQLx compile-time checks.
For running Cargo on the host, `exposed/config/dev.yaml` uses `localhost:55432`.

Use `docker compose stop exposed` to stop the app and keep the caches.
`make db-start` and `make db-stop` control only PostgreSQL.

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
| `make db-revert` | Revert the baseline, deleting its tables and data |
| `make db-migration-status` | Show SQLx migration status |
| `make db-nuke` | Delete the configured database and recreate it from the baseline |
| `make initialize` | Initialize members and declarations; safe to repeat |
| `make import-members` | Manually refresh member data through Rust |
| `make import-declarations` | Refresh declarations for the stored member cohort |
| `make verify` | Check data in the local Compose database |
| `make check` | Run lint, formatting checks, type checks and all tests |
| `make test-unit` | Run tests without PostgreSQL |
| `make format` | Format Python files |
| `make db-stop` | Stop local PostgreSQL while retaining data |

`make test-rust` and `make check` require local PostgreSQL (`make db-start`). Rust tests create and remove their own temporary databases. Override `RUST_DATABASE_URL` with an initialized PostgreSQL URL to use another server; it must allow database creation and include the `exposed` search path for existing SQLx query macros. `SQLX` can also be overridden with an absolute CLI path. Python tests need no database.

See the [importer documentation](ingest/README.md) for configuration, data rules and refresh behavior.

## Refresh status and optional WhatsApp

Host development uses `exposed/config/dev.yaml` with a loopback-only operator listener.
Run `cargo run -- exposed/config/dev.yaml` after Python setup and migrations.
Set `DATABASE_URL` for SQLx compile-time queries (see `RUST_DATABASE_URL` in the Makefile).
The application starts its timers and catches up when a refresh is due.
Inspect `GET /imports/status` on port 7000 (include the bearer token for Compose).
`last_completed`, `outcome`, `rejected`, `new_members` and `next_attempt` explain freshness.

WhatsApp is disabled by default. Add this under `imports` only after provisioning
a Business Platform sender and an approved template with one body text parameter:

```yaml
# Choose notification_interval_seconds later; omitting it queues no notifications.
whatsapp:
  api_version: vXX.Y             # supported version chosen during setup
  phone_number_id: 'SENDER_ID'
  recipient: 'RECIPIENT_DIGITS'
  template: exposed_refresh
  language: en_GB
  token_env: WHATSAPP_ACCESS_TOKEN
```

Supply the access token through the application's environment. Notification timing
is an application setting, independent of WhatsApp and import retry timing. A durable
queue retries delivery without changing import results. Setup never sends a message.
