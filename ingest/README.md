# Exposed

Import everyone who has served in the Commons during the configured current Parliament into PostgreSQL. Refreshes keep latest profiles, dated service and former members.

## Run locally

Requires Python **3.14+** (for UUIDv7), Docker and Docker Compose.

```sh
python3 -m venv .venv
.venv/bin/python -m pip install -r requirements.lock
.venv/bin/python -m pip install --no-deps -e .
cp .env.example .env
docker compose up -d --wait
.venv/bin/python -m exposed migrate
.venv/bin/python -m exposed import-members
```

Run these commands from this directory. Keep an existing `.env` when repeating setup. The importer loads only this directory's `.env`; existing environment variables take precedence. Compose exposes its development database on **127.0.0.1:55432** and stores data in a named volume.

Repeat `.venv/bin/python -m exposed import-members` to refresh. Member and service IDs remain stable across unchanged runs. Progress goes to stderr; stdout contains a JSON summary with member/current/former counts, inserted/updated/unchanged profiles and service periods. A successful run exits `0`, an import failure `1`, invalid CLI configuration `2`, and an interrupted import `130`.

```sh
# Read-only verification, using the Postgres client inside the container.
docker compose exec -T postgres psql -U exposed -d exposed < scripts/verify.sql

# Stop the local server while keeping its data.
docker compose stop
```

## Configuration

| Setting | Meaning |
| --- | --- |
| `DATABASE_URL` | PostgreSQL connection URL, including any required TLS settings such as `sslmode=require`. Required. |
| `PARLIAMENT_TERM_START` | The term's start date: election day, currently `2024-07-04`. Required for imports unless passed with `--term-start`. |
| `--term-start YYYY-MM-DD` | Explicit override for the import command. |

For a hosted database, set `DATABASE_URL` to that database and run the same migration and import commands. **Run only one import or migration against a database at a time.** This is an operating assumption; the application does not coordinate concurrent jobs. Apply that same restriction in the scheduler when scheduling is added.

The command is ready to run in GitHub Actions with `DATABASE_URL` supplied through a secret and `PARLIAMENT_TERM_START` through configuration. No scheduling or deployment is configured in this milestone. The importer uses UK local time for each run's fixed `as_of` date; it does not offer a historical-as-of mode because profiles and current membership come from the live API.

## Data and date rules

All tables live in the `exposed` schema. Internal IDs are UUIDv7; `members.parliament_member_id` is Parliament's stable numeric ID and the future join key for interests.

| Table | Stores |
| --- | --- |
| `parliament_terms` | `id`, `term_start` and nullable `term_end` (unknown while ongoing). |
| `members` | Latest name, party and House/membership location; current Commons status. |
| `member_terms` | One row per source Commons service period in the term, including its original start/end dates and the term-specific dates. Multiple rows preserve a member's departure and re-entry. |
| `schema_migrations` | Applied SQL migration versions and checksums. |

`served_from = max(source_start_date, term_start)`. `served_until` preserves the source service end, including `null` for an open period. End dates are treated as the dates service ceased; an interval ending on election day does not create service in the new term. A database constraint and trigger enforce valid service dates.

`latest_house` describes the latest profile. A former MP who has since joined the Lords remains in the cohort with Commons service and `is_current_commons = false`. `latest_membership_from` is a constituency for a latest Commons profile; it should not be treated as a historical constituency or always as a constituency for Lords profiles. Dedicated party/constituency history tables are outside this milestone.

This version refreshes **one explicitly configured current Parliament**. It refuses a changed term start once that database has imported a term. `term_end` must be on or after `term_start` when set. Refreshes preserve its stored value; this importer does not populate it automatically. Term rollover and closed-Parliament backfills require a future extension. It does not detect a new general election: stop scheduled imports when this Parliament ends, and add rollover support before resuming. The configured date is a lower boundary, so leaving an old date in place would include later service too.

## Refresh guarantees

1. Open **one database transaction for the whole refresh**.
2. Fetch current Commons membership pages, keeping their member IDs in memory.
3. Read the historical Commons cohort one page at a time using `MembershipInDateRange`. Fetch that page's service histories, validate the data, and immediately write its member profiles and service periods. No current/latest-House/eligibility filter is applied to the historical cohort.
4. Reconcile corrected service periods and, after the last page, check completeness and that no previously imported historical member has disappeared. Commit once. Former members are kept.
5. If any request, validation or database write fails, the transaction rolls back **all batches** and the command reports the failure. Other database sessions see the previous completed data until commit.

The transaction stays open during API requests and retries. PostgreSQL manages the transaction's normal write locks and releases them when it ends. The importer keeps ID sets and the current page in memory; it does not assemble a complete copy of the import before writing it.

Requests are sequential with a short delay, timeouts and up to four attempts for transport errors, HTTP 429 and server errors. API data can change between requests. Validation detects incomplete pagination and inconsistent membership, but the fixed observation date does not freeze the upstream data. Rerun after source data settles if validation fails.

API responses and import execution history are not stored. Summaries and failures are command output only.

For an existing installation, run `.venv/bin/python -m exposed migrate` before importing. Migration `002_remove_audit_add_term_end.sql` adds the nullable column and deletes the audit tables and their references, preserving members, terms and service records. The original migration stays unchanged so existing migration checksums remain valid.

## Tests

```sh
.venv/bin/python -m ruff check src tests
.venv/bin/python -m ruff format --check src tests
.venv/bin/python -m pyright
.venv/bin/python -m pytest -q -m 'not integration'

# Full suite, including actual transaction/rollback checks against local Postgres.
EXPOSED_TEST_ADMIN_DSN=postgresql://exposed:exposed_local_dev@localhost:55432/postgres \
  .venv/bin/python -m pytest -q
```

Integration tests create and drop their own randomly named `exposed_test_*` databases; the test role needs `CREATE DATABASE`. They do not modify the application database. Without `EXPOSED_TEST_ADMIN_DSN`, integration tests are explicitly skipped. All API responses in automated tests use synthetic fixtures; a live import is a separate end-to-end check.

See the [recorded verification results](../docs/verification/member-import.md) for automated and live import checks.

### Neovim and type checking

Ruff provides linting and formatting. Use Pyright alongside it for type checking; Ruff does not check whether values match their annotated types. The `[tool.pyright]` section in `pyproject.toml` selects `ingest/.venv` relative to that file and checks both `src` and `tests`. Pyright is included in the development dependencies and pinned in `requirements.lock`.

Astral also provides `ty`, a separate type checker and language server with Neovim support. This project currently verifies types with Pyright. See the [Ruff comparison](https://docs.astral.sh/ruff/faq/#how-does-ruff-compare-to-mypy-or-pyright-or-pyre) and [ty editor documentation](https://docs.astral.sh/ty/editors/).

## Implementation map

- `src/exposed/api.py`: HTTP requests, page iteration, retries.
- `src/exposed/models.py`: source validation and term-specific service dates.
- `src/exposed/db.py` and `migrations/`: schema and SQL operations.
- `src/exposed/importer.py`: batch processing, the transaction boundary.
- `src/exposed/cli.py`: configuration and command output.
- [API research](../docs/research/uk-parliament-apis.md): source contracts, historical coverage and future interests ingestion.

Contains Parliamentary information licensed under the [Open Parliament Licence v3.0](https://www.parliament.uk/site-information/copyright-parliament/open-parliament-licence/).
