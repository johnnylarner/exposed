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
```

Setup creates `ingest/.env` only when absent. The Makefile runs SQLx from `ingest/`
so it and the importer use that file; existing environment variables take
precedence. The example URL targets local PostgreSQL and explicitly disables TLS
for that loopback connection. Use the appropriate TLS setting for a hosted database.

The schema uses a single development baseline. See the
[migration instructions](db/README.md) before adopting SQLx on an existing dbmate
database or editing the baseline. See the
[funder identification rules](ingest/README.md#funder-identification) for imported funding.

## Develop the application with Docker Compose

Initialize the database using the setup above before starting the app. SQLx checks
queries against the live schema during compilation, so a fresh database needs
`make db-migrate` first. Migrations remain an explicit step.

```sh
docker compose --env-file .env.compose up --build
```

With the example ports, open the Svelte frontend at **http://localhost:5173**. It searches the existing
API as you type, shows MPs and funders together in the API's similarity order,
and preserves the names returned by the source. Enter at least three characters;
press `/` to focus search and Escape to clear it. A query URL such as
`http://localhost:5173/?q=HSBC` can be shared for manual testing.

With the example ports, the API listens at `http://localhost:6999`. For example:

```sh
curl 'http://localhost:6999/search?term=McDonald&max_entries=10'
```

The repository is bind-mounted into the container. `cargo watch` rebuilds and
restarts the app when Rust source, Cargo manifests, `Cargo.lock`, or files in
`exposed/config/` change. Polling detects edits through Docker Desktop bind mounts.
The first image and app builds take longer; named volumes cache Cargo downloads
and Linux build artifacts separately from the host's `target/` directory.

The server reads its configuration from environment variables and an optional `.env` file at startup.
It uses `.env` from the current directory or the nearest parent directory that contains one.
Exported environment variables take precedence over values in `.env`.
Only `DATABASE_URL` is required.

The server listens on `0.0.0.0:6999` by default.
If port `6999` is occupied, the operating system selects an available port.
The server prints the selected address at startup.

Compose sets `DATABASE_URL` to connect to `postgres:5432`.
SQLx uses the same variable for query checks at compile time.

To run the server on the host, create `.env` from the local database example:

```sh
test -f .env || cp .env.example .env
cargo run --package exposed --bin exposed -- server
```

Edit `DATABASE_URL` in `.env` to use another database. Git ignores `.env`.

The `data members load` and `data declarations fetch` commands also read
`DATABASE_URL` from the environment or `.env`, with the same precedence as the server.
Their YAML files contain only data settings such as `data_dir` and `batch_size`.
Remove `connection_string` from existing YAML files and set `DATABASE_URL` in `.env` instead.
The `data members fetch` command does not require a database URL.

To fetch and load members, use the same ingestion key for both stages:

```sh
test -f .env || cp .env.example .env
ingestion_key=$(uuidgen)
cargo run --package exposed --bin exposed -- data members fetch exposed/config/cli-dev.yaml --ingestion-key "$ingestion_key"
cargo run --package exposed --bin exposed -- data members load exposed/config/cli-dev.yaml --ingestion-key "$ingestion_key"
cargo run --package exposed --bin exposed -- data declarations fetch exposed/config/declarations-dev.yaml
```

To inspect the latest stored ingestion run, use either fetch configuration:

```sh
cargo run --package exposed --bin exposed -- data latest exposed/config/cli-dev.yaml
```

The command prints the ingestion key and absolute path of the most recently
modified run directory. It considers only directories with UUID names. Equal
timestamps use the greatest UUID.

This command requires only `data_dir` in the YAML file and no database connection
at runtime. It does not change files. An absent or empty data directory produces
`No ingestion runs found.` and a successful exit.

The frontend service proxies `/api/search` to `http://exposed:6999/search`.
Its source is bind-mounted, with polling for live reload on Docker Desktop and a
separate Linux dependency volume. Restart `frontend` after changing its package
manifest or lockfile; dependencies are refreshed at startup. The API may take
longer than the frontend to compile on a first run; use **Try again** if search
is not ready yet.

Use `docker compose --env-file .env.compose stop frontend exposed` to stop the app and keep the caches.
`make db-start` and `make db-stop` control only PostgreSQL.

### Separate worktree environments

From the worktree root, create a Compose environment with three free host ports:

```sh
python3 .agents/skills/setup-env/scripts/setup-compose.py
test -f .env || cp .env.example .env
```

The script writes `.env.compose` with `FRONTEND_PORT`, `API_PORT`, and
`POSTGRES_PORT`. It derives `COMPOSE_PROJECT_NAME` from the worktree path and
branch, so each worktree has separate containers, networks, and volumes.
Git ignores `.env.compose`. Rerunning the script selects new free ports and
preserves the existing project name and other settings. Run Compose again to
apply the new ports to an existing deployment.

Set `DATABASE_URL` in `.env` to the value that the script prints.
For Python imports or `make db-migrate`, set the same value in `ingest/.env`.
For importer integration tests, set `EXPOSED_TEST_ADMIN_DSN` to that URL with
`/postgres` in place of `/exposed`.

Start PostgreSQL and apply the schema before the application compiles:

```sh
docker compose --env-file .env.compose up -d --wait postgres
sqlx migrate run
docker compose --env-file .env.compose up -d
```

Use the frontend and API URLs that the script prints. Container ports remain
5173, 6999, and 5432. The host ports come from `.env.compose`.
The Makefile uses `.env.compose` when present and `.env.compose.example` otherwise.
Direct Compose commands require `--env-file .env.compose` to read the generated file.
Exported shell variables take precedence over the file, as described in the
[Docker Compose documentation](https://docs.docker.com/compose/how-tos/environment-variables/variable-interpolation/).

The ports are free when the script selects them. Another process can claim them
before Compose starts. To select new ports, run the script again.
Then update `DATABASE_URL` to match the new PostgreSQL port.

### Frontend development and checks

To run the frontend on the host instead of Docker, use Node.js **24+**:

```sh
cd frontend
npm ci
npm run dev
```

The host dev server proxies to `http://127.0.0.1:6999` by default. Override
`EXPOSED_API_ORIGIN` when starting Vite to use another backend. This setting is
server-side; the browser always uses the same-origin `/api/search` path.
The frontend image is for local development. `npm run build` produces static
assets in `frontend/dist`; a production host must also route `/api/search` to
the Rust API.

The similarity threshold slider refreshes results automatically: lower values
include broader matches, from 0 to 1 in steps of 0.01 (default 1). The search URL
preserves the query and threshold, for example `/?q=John&strictness=0.65`.
Search delay, result limits and defaults live in
[`frontend/src/lib/search.ts`](frontend/src/lib/search.ts). The API caps each entity type separately, so the UI describes the returned
results without claiming a total count or offering unsupported pagination.
This slice displays results; entity pages and in-app feedback collection are
future work. Feedback is gathered through manual testing.

For frontend checks, install dependencies and the browser once, then run:

```sh
npm --prefix frontend ci
cd frontend && PLAYWRIGHT_BROWSERS_PATH=0 npx playwright install chromium && cd ..
make frontend-check
```

Browser tests use a controlled API to check ranking, request cancellation,
empty/error recovery, text escaping and narrow layouts. For manual verification,
try an MP's name, `HSBC`, a misspelling, clearing a pending query, and stopping
the API to check retry behavior. See the
[search design brief](docs/design/entity-search.md) for the agreed scope.

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
| `make import-members` | Refresh member data |
| `make import-declarations` | Refresh declarations for the stored member cohort |
| `make verify` | Check data in the local Compose database |
| `make check` | Run importer lint, formatting checks, type checks and tests |
| `make frontend-dev` | Start the Svelte frontend and API with live reload |
| `make frontend-check` | Check, build and browser-test the frontend |
| `make test-unit` | Run tests without PostgreSQL |
| `make format` | Format Python files |
| `make db-stop` | Stop local PostgreSQL while retaining data |

`make test` and `make check` require local PostgreSQL (`make db-start`). Tests create and remove their own temporary databases. Override `EXPOSED_TEST_ADMIN_DSN` with a PostgreSQL URL to use a different test server; it must allow database creation. `SQLX` can also be overridden with an absolute SQLx CLI executable path.

See the [importer documentation](ingest/README.md) for configuration, data rules and refresh behavior.
