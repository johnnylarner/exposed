# exposed

Where do MPs get their money from?

Use `exposed data declarations clean --help` for stage-specific YAML fields,
examples, and environment requirements. Use `--schema` on a data stage or utility
to discover its generated success and failure JSON Schema response contracts.
Schema inspection needs no configuration or database and writes no files:

```sh
exposed data declarations clean --schema
exposed data util latest --schema
exposed data util uuid-v7 --schema
```

Schemas come from handler response types. `exposed --version` prints the version.
Declaration stages run in order: `fetch`, `clean`, `resolve`, then `load`. Member
stages run `fetch` then `load`. Fetch can generate a fresh UUID-v7 when
`--ingestion-key` is omitted. Every later stage requires an explicit existing run
key and `--config`. Put the stage before its flags, as in the examples above.
Relative `data_dir` paths resolve from the working directory.

Data commands write one JSON document to stdout on success. Command failures
write `{"error":{"message":"..."}}` to stderr and exit with code 1. Help and
argument parsing use Clap's standard output.

## Server healthcheck

`exposed server` listens on port 6999 and fails if that port is already in use.
`GET /healthz` returns HTTP 204 with an empty body while the HTTP server responds.
This is a liveness check. It does not query the database or establish readiness
for search and detail requests.

Run `exposed healthcheck` to probe `http://127.0.0.1:6999/healthz`. The command
needs no configuration or `DATABASE_URL` and does not load `.env`. It bypasses
proxies, rejects redirects and every status except 204, and times out after two
seconds. Success exits with code 0 and no output. Failure exits with code 1 and
writes an error to stderr.

The production `binary` image runs this probe every 30 seconds. The development
Compose service runs the compiled debug binary and allows 120 seconds for its
initial compilation before counting failures.

## Development hooks

Enable the repository's pre-commit hook after cloning:

```sh
git config core.hooksPath .githooks
```

The hook runs `cargo clippy --workspace --all-targets --all-features -- -D warnings`
and blocks the commit if Clippy fails. It checks the working tree, including
unstaged changes. Configure `DATABASE_URL` in the root `.env` or environment and
apply the [database migrations](db/README.md) before committing. SQLx needs the
database to check queries in test targets.

To fetch and load members, use the same ingestion key for both stages:

```sh
test -f .env || cp .env.example .env
ingestion_key=$(cargo run --quiet --package exposed --bin exposed -- data util uuid-v7 | jq -r .uuid)
cargo run --package exposed --bin exposed -- data members fetch --config exposed/config/cli-dev.yaml --ingestion-key "$ingestion_key"
cargo run --package exposed --bin exposed -- data members load --config exposed/config/cli-dev.yaml --ingestion-key "$ingestion_key"
cargo run --package exposed --bin exposed -- data declarations fetch --config exposed/config/declarations-dev.yaml
```

Choose the utility command for your task:

| Command | When to use it |
| --- | --- |
| `data util uuid-v7` | Preselect an ingestion key to share across fetch and load stages, or across reprocessing stages. |
| `data util latest` | Find the ingestion key and directory of the most recently modified local run before inspecting its artifacts or running a pipeline stage. It reads the store without changing it. |
| `data util copy-latest-raw` | Reprocess a completed capture after changing cleaning or resolution logic, or try different resolution settings. It creates a separate run with the same raw inputs and no derived results. |

Use `data members fetch` or `data declarations fetch` when you need fresh data
from Parliament. These utilities do not fetch data or run pipeline stages. After a
copy, pass the destination ingestion key to each stage you want to run.

To generate a fresh UUID-v7:

```sh
cargo run --package exposed --bin exposed -- data util uuid-v7
```

The command returns one JSON object, for example
`{"uuid":"0199e6c5-2d80-7a12-8b34-56789abcdef0"}`. It needs no configuration or
database connection and writes no files. Use the `uuid` value as `--ingestion-key`
for stages that should share a run.

To inspect the latest stored ingestion run, use either fetch configuration:

```sh
cargo run --package exposed --bin exposed -- data util latest --config exposed/config/cli-dev.yaml
```

The command returns `{"ingestion":{"ingestion_key":"...","path":"..."}}`
with the key and absolute path of the most recently
modified run directory. It considers only directories with UUID names. Equal
timestamps use the greatest UUID.

This command requires only `data_dir` in the YAML file and no database connection
at runtime. It does not change files. An existing empty data directory produces
`{"ingestion":null}` and a successful exit. A missing or unreadable directory
fails with its path in the error.

To reprocess the latest raw capture in a new run, copy it with a fresh ingestion key:

```sh
ingestion_key=$(cargo run --quiet --package exposed --bin exposed -- data util uuid-v7 | jq -r .uuid)
cargo run --package exposed --bin exposed -- data util copy-latest-raw --config exposed/config/declarations-dev.yaml --ingestion-key "$ingestion_key"
cargo run --package exposed --bin exposed -- data declarations clean --config exposed/config/declarations-dev.yaml --ingestion-key "$ingestion_key"
```

Omit `--ingestion-key` to generate a UUID-v7. The command returns an `ingestion`
object with `source_ingestion_key`, `ingestion_key`, and the absolute destination
run `path`. It requires only `data_dir`
and runs without a database or API connection.

The command uses the same latest-run selection as `data util latest`. It copies the
entire `raw` directory, preserving file bytes and empty directories. It excludes
cleaned and resolved results and fails if the selected run has no raw directory.
Symlinks and special files fail the copy. Existing destinations remain unchanged.
The complete run appears only after the copy succeeds. An existing empty store
returns `{"ingestion":null}` without creating directories. A missing store fails with a path-bearing error.
Copy only captures
that are no longer being written.

To split a captured declaration run into funding occurrences and funder
observations, provide its ingestion key explicitly:

```sh
cargo run --package exposed --bin exposed -- data declarations clean --config exposed/config/declarations-dev.yaml --ingestion-key "$ingestion_key"
```

Cleaning runs offline and requires only `data_dir` in the configuration. It writes
`<data_dir>/<ingestion_key>/cleaned/declarations/funding_entries.parquet` and
`funders.parquet` together. It preserves raw files and refuses an existing cleaned
result. Funder rows are source observations, with separate donor, payer, and
ultimate-payer roles. They include parallel name features for later entity
resolution. Matching names do not merge observations. Declaration-level names
remain available even when the declaration has no funding entry. See the
[declaration cleaning reference](docs/declaration-cleaning.md) for table semantics
and exact feature rules.

Search result names open MP and funder detail pages. MP summaries show known declared values across all stored history, separated by
currency, category breakdowns, and the five latest declarations across all categories.
The declaration history page shows every stored declaration with registration date
filters and pages of ten declarations, including incomplete funding entries. Funders show declared
support by recipients' latest stored party and up to ten recipients per currency.
These party groups do not establish membership when support was received or direct
payments to parties. Unknown currencies are excluded from totals; unknown amounts
are reported beside partial totals. Values remain exact decimal strings. See the [entity details reference](docs/entity-details.md)
for response semantics and read ownership.

The HTTP API exposes `/search`, `/members/{Parliament ID}`, and `/funders/{UUID}`.
Search entities include stable `id` strings interpreted by `kind`. Detail pages
preserve search query and strictness in native links, so Back, reload, and new tabs
work normally. SvelteKit renders initial search and detail pages on the server.

The frontend server reads the Rust API at `http://exposed:6999` inside Compose.
Only the frontend port is published; browser navigation uses SvelteKit page data.
Its source is bind-mounted, with polling for live reload on Docker Desktop and a
separate Linux dependency volume. Restart `frontend` after changing its package
manifest or lockfile; dependencies are refreshed at startup. The API may take
longer than the frontend to compile on a first run; use **Try again** if search
is not ready yet.

Use `docker compose stop frontend exposed` to stop the app and keep the caches.
`make db-start` and `make db-stop` control only PostgreSQL.

### Refresh the search regression database

To replace `exposed_search_regression` with a fresh clone of `exposed`, stop the
app and disconnect other clients from `exposed`. Run from the repository root:

```sh
docker compose stop exposed
./scripts/refresh-search-regression-db.sh
docker compose start exposed
```

The script requires Docker Compose and a running `postgres` service. You can
invoke the script by its path from any directory. It uses the repository's
Compose file and honors `COMPOSE_PROJECT_NAME`.

Each refresh deletes the previous target and terminates its connections. If the
source is busy and cloning fails, the previous target remains unchanged. The
script never terminates source connections or changes `exposed`. It reserves
`exposed_search_regression_stage` as disposable storage and deletes any stale
stage before cloning. Concurrent refresh attempts fail without changing either
database. The script does not stop or restart the app.

The Compose project is named `exposed`. When testing a frontend-only change in a
worktree against an already running API, use
`docker compose up --build -d --no-deps frontend` to avoid recreating the backend
and database from that worktree. This serves the frontend from the worktree.

### Frontend development and checks

To run the frontend on the host instead of Docker, use Node.js **24+**:

```sh
cd frontend
npm ci
npm run dev
```

The frontend server reads `http://127.0.0.1:6999` by default. Set the private
`EXPOSED_API_ORIGIN` environment variable to select another backend. For the
Compose backend, run the frontend inside Compose because the Rust port is private.
Browser requests never call the Rust API directly.

`npm run build` produces an adapter-node server in `frontend/build`. Run it with
`EXPOSED_API_ORIGIN=http://127.0.0.1:6999 node build` from `frontend`.
Set `HOST` and `PORT` for the listening address. The release Docker target
`frontend-server` runs only the Node frontend on port 8080. Deploy the Rust server
separately and set the frontend's private `EXPOSED_API_ORIGIN` to its reachable
origin. The frontend image does not require `DATABASE_URL`.

The similarity threshold slider refreshes results automatically: lower values
include broader matches, from 0 to 1 in steps of 0.01 (default 0.8). The search URL
preserves the query and threshold, for example `/?q=John&strictness=0.65`.
Search delay, result limits and defaults live in
[`frontend/src/lib/search.ts`](frontend/src/lib/search.ts). The API caps each entity type separately, so the UI describes the returned
results without claiming a total count or offering unsupported pagination.
The API also defaults to `0.8` when `strictness` is omitted. Explicit thresholds
control primary trigram matches and corrected fallback similarity. When primary
matches contain no MP or Individual funder,
search appends phonetic person matches after the primary results. Organization
matches do not suppress this fallback. Each query word is corrected against
each candidate independently, choosing the
closest name word with the same Soundex code and at most one edit. Name words
can be reused. Every query word must match, and the corrected query must meet
the requested `word_similarity` threshold against that candidate. Extra name
words are allowed; words from different aliases cannot combine. Fallback
results rank by total edits, then corrected similarity and stable identity.
Canonical names win alias ties. Primary and fallback funders share the per-type result limit.

In-app feedback collection is future work. Feedback is gathered through manual
testing.

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

## Resolve declaration funders

Resolution uses the native SIMD scorer from `weldrs` 0.2.2 and requires no Python setup. Configure the data directory and candidate budget using the [resolution guide](resolution/README.md), then resolve a cleaned run from the repository root:

```sh
cargo run --package exposed --bin exposed -- data declarations resolve --config exposed/config/declaration-resolution-dev.yaml --ingestion-key "$ingestion_key"
```

The command writes observation identities, one attribution per funding occurrence,
statistical candidate decisions, and a manifest under `resolved/declarations`.
Company anchors use captured source numbers. The frozen native model supplies uncalibrated scores
for observations without eligible company IDs, including noncompanies. Initial
automatic links require exact names and complete usable addresses. Name-only and
fuzzy candidates remain for review, while named singletons receive separate
provisional identities. See the [resolution policy](docs/funder-resolution-design.md).

Load a resolved run into the searchable database after its members have been loaded:

```sh
cargo run --package exposed --bin exposed -- data declarations load --config exposed/config/declarations-dev.yaml --ingestion-key "$ingestion_key"
```

The loader includes declarations with no funding occurrences. It replaces only
declarations whose captured timestamps are newer. Every successful load refreshes
stored funder groups, direct evidence, and preferred names. An exact retry of an
imported run refreshes those groups while preserving source data. Keep the raw,
cleaned, and resolved files for the ingestion key because the loader validates
and fingerprints all three stages. It
replaces each incoming funder's name, kind, company number, and searchable aliases
with that run's resolved values, including missing optional values. A shared
identity keeps its database ID, but its metadata and aliases reflect the last
accepted run. Declarations omitted from the run retain their funding entries.
After replacing entries, the loader removes funders and aliases that no remaining
funding entry references. These changes commit together or all roll back.

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

## Production image

See [Build and run the production image](docs/production-image.md) for standalone
Docker builds, database setup, SQLx metadata updates, and main-branch releases.

## Tracing

The server writes structured JSON traces to stderr. Set `EXPOSED_LOG_FILE` to
append traces to a separate file for any command, including the server.
CLI tracing has no output sink unless you set this variable. Data and migration
commands keep their JSON response contract even when tracing is enabled.

```sh
EXPOSED_LOG_FILE=exposed.jsonl cargo run --bin exposed -- data util uuid-v7
RUST_LOG=off,exposed=debug cargo run --bin exposed -- server
```

`RUST_LOG` controls filtering and never selects the CLI output sink. The default
filter enables application events at INFO and above and disables dependency
events. Enabling dependency events can add their own error reports.

HTTP request middleware emits one ERROR for each 4xx or 5xx response, including
routing and extraction failures. It records method, route template, status,
duration, and available private error diagnostics. Other responses produce INFO
completion events. Request URLs, queries, and bodies are not recorded.

CLI response emission logs command and serialization failures once. The process
boundary logs failures that escape dispatch, such as startup or output writes.
A command failure followed by a failed error-response write produces two events
because two distinct failures occurred. Services and repositories propagate
errors without logging them.

Trace files use synchronous append writes. The application does not rotate them.
An unavailable trace file fails command startup. Help and data schema inspection
bypass tracing setup and do not open the configured file.
