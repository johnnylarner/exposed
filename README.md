# exposed

Where do MPs get their money from?

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

To split a captured declaration run into funding occurrences and funder
observations, provide its ingestion key explicitly:

```sh
cargo run --package exposed --bin exposed -- data declarations clean exposed/config/declarations-dev.yaml --ingestion-key "$ingestion_key"
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

The frontend service proxies `/api/search` to `http://exposed:6999/search`.
Its source is bind-mounted, with polling for live reload on Docker Desktop and a
separate Linux dependency volume. Restart `frontend` after changing its package
manifest or lockfile; dependencies are refreshed at startup. The API may take
longer than the frontend to compile on a first run; use **Try again** if search
is not ready yet.

Use `docker compose stop frontend exposed` to stop the app and keep the caches.
`make db-start` and `make db-stop` control only PostgreSQL.

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

## Resolve declaration funders

Install the pinned Python worker with the [resolution setup](resolution/README.md). Then resolve a cleaned run from the repository root:

```sh
cargo run --package exposed --bin exposed -- data declarations resolve exposed/config/declaration-resolution-dev.yaml --ingestion-key "$ingestion_key"
```

The command writes observation identities, one attribution per funding occurrence,
statistical candidate decisions, and a manifest under `resolved/declarations`.
Company anchors use captured source numbers. Splink supplies uncalibrated scores
for observations without eligible company IDs, including noncompanies. Initial
automatic links require exact names and complete usable addresses. Name-only and
fuzzy candidates remain for review, while named singletons receive separate
provisional identities. See the [resolution policy](docs/funder-resolution-design.md).

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

