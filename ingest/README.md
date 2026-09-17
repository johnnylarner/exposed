# Exposed

Import Commons members and their available financial declarations into PostgreSQL. Member refreshes keep latest profiles, dated service and former members; declaration refreshes store parsed declaration fields and funding independently.

## Run locally

From the repository root, follow the [setup instructions](../README.md), then use `make import-members` to refresh. The development schema uses [dbmate](../db/README.md) through `make db-migrate`.

Member and service IDs remain stable across unchanged imports. Progress goes to stderr; stdout contains a JSON summary with member/current/former counts, inserted/updated/unchanged profiles and service periods. A successful import exits `0`, an import failure `1`, invalid CLI configuration `2`, and an interrupted import `130`.

Both the Makefile's dbmate commands and the importer load `ingest/.env`; existing environment variables take precedence. For direct CLI use, run `.venv/bin/python -m exposed import-members` from `ingest/`.

## Configuration

| Setting | Meaning |
| --- | --- |
| `DATABASE_URL` | PostgreSQL connection URL, including any required TLS settings such as `sslmode=require`. Required. |
| `PARLIAMENT_TERM_START` | The term's start date: election day, currently `2024-07-04`. Required for imports unless passed with `--term-start`. |
| `--term-start YYYY-MM-DD` | Explicit override for the import command. |

For a hosted database, set `DATABASE_URL` to that database and run the same migration and import commands. **Run only one import or migration against a database at a time.** This is an operating assumption; the application does not coordinate concurrent jobs. Apply that same restriction in the scheduler when scheduling is added.

The command is ready to run in GitHub Actions with `DATABASE_URL` supplied through a secret and `PARLIAMENT_TERM_START` through configuration. No scheduling or deployment is configured in this milestone. The importer uses UK local time for each run's fixed `as_of` date; it does not offer a historical-as-of mode because profiles and current membership come from the live API.

## Data and date rules

Domain tables live in the `exposed` schema; dbmate keeps migration versions in `public.schema_migrations`. Internal IDs are UUIDv7; `members.parliament_member_id` is Parliament's stable numeric ID and the source identity used to retrieve declarations.

| Table | Stores |
| --- | --- |
| `parliament_terms` | `id`, `term_start` and nullable `term_end` (unknown while ongoing). |
| `members` | Latest name, party and House/membership location; current Commons status. |
| `member_terms` | One row per source Commons service period in the term, including its original start/end dates and the term-specific dates. Multiple rows preserve a member's departure and re-entry. |

`served_from = max(source_start_date, term_start)`. `served_until` preserves the source service end, including `null` for an open period. End dates are treated as the dates service ceased; an interval ending on election day does not create service in the new term. A database constraint and trigger enforce valid service dates.

`latest_house` describes the latest profile. A former MP who has since joined the Lords remains in the cohort with Commons service and `is_current_commons = false`. `latest_membership_from` is a constituency for a latest Commons profile; it should not be treated as a historical constituency or always as a constituency for Lords profiles. Dedicated party/constituency history tables are outside this milestone.

This version refreshes **one explicitly configured current Parliament**. It refuses a changed term start once that database has imported a term. `term_end` must be on or after `term_start` when set. Refreshes preserve its stored value; this importer does not populate it automatically. Term rollover and closed-Parliament backfills require a future extension. It does not detect a new general election: stop scheduled imports when this Parliament ends, and add rollover support before resuming. The configured date is a lower boundary, so leaving an old date in place would include later service too.

## Refresh guarantees

1. Open **one database transaction for the whole refresh**.
2. Fetch current Commons membership pages. Within this stream, log and collapse identical profiles for a repeated member ID; reject conflicting profiles.
3. Read the historical Commons cohort one page at a time using `MembershipInDateRange`. Apply the same duplicate checks across all candidate pages, then fetch histories for the remaining members, validate the data, and immediately write their profiles and service periods. No current/latest-House/eligibility filter is applied to the historical cohort.
4. Reconcile corrected service periods and, after the last page, check that all current members appear in the historical results. Commit once. Previously stored members absent from the response are kept unchanged.
5. If any request, validation or database write fails, the transaction rolls back **all batches** and the command reports the failure. Other database sessions see the previous completed data until commit.

The transaction stays open during API requests and retries. PostgreSQL manages the transaction's normal write locks and releases them when it ends. The importer retains profiles to compare duplicates within each source stream and processes service histories one batch at a time. Identical repeats do not add writes or inflate summary counts; conflicting repeats roll back the entire member refresh.

Requests are sequential with a short delay, timeouts and up to four attempts for transport errors, HTTP 429 and server errors. API data can change between requests. Validation detects incomplete pagination and inconsistent membership, but the fixed observation date does not freeze the upstream data. Rerun after source data settles if validation fails.

Source responses and import execution history are not stored. Summaries and failures are command output only.

## Declaration ingestion

After migrating and importing members, run `make import-declarations`, or
`.venv/bin/python -m exposed import-declarations --term-start 2024-07-04` from `ingest/`.
The command uses the same `DATABASE_URL` and `PARLIAMENT_TERM_START` configuration.
It reads distinct members with Commons service in the stored term, including former MPs.
It does not create or refresh members or terms; an absent matching cohort is a configuration failure.

Requests explicitly select Commons, all categories, all available registers and expired declarations.
The term determines which members are queried, without limiting declaration dates. Coverage is the
history returned by Parliament's API, not a guarantee of a complete lifetime archive. Child payments
are separate declarations; parent details are resolved during ingestion when needed for attribution.

| Table | Stores |
| --- | --- |
| `declarations` | Stable UUIDv7, unique API declaration ID, member FK, category ID and readable name, source registration date, and retrieval time. |
| `funding_entries` | UUIDv7, declaration source ID, nullable source funder, exact numeric amount, currency and payment type. |

A declaration can have zero or many funding rows. Names are source attributions; no shared donor
identity is inferred. Funding is extracted from the version with the latest `register.publishedDate`.
`registration_date` comes from that version's `registrationDate`, independently of `fetched_at`.
Parliament supplies a calendar date, not a creation time of day. Missing source dates stay null;
publication dates and retrieval timestamps are not substituted.
New declaration ingestions populate these fields directly. After recreating the development
schema, run `make import-members` followed by `make import-declarations`.
Raw API responses and unused source fields are not stored. Equally recent versions with conflicting
content are rejected. Declaration amounts are not comparable donation totals:
ongoing earnings, in-kind valuations and other categories have different meanings.

The parser supports direct `Value` fields and nested `Donors` groups, preserving each donor/amount
pair. It prefers an explicit `UltimatePayerName`, then source donor/payer names, resolving required
parent data through the API when necessary. A required parent with no usable payer rejects the child;
an explicitly different but withheld ultimate payer remains absent. Nonfinancial declarations have no invented funding.
Absent optional values stay null. Ordinary decimal strings and integer values are parsed exactly;
unsupported numeric formats are logged for later parser improvements. Unrecognized currency-bearing
fields or monetary fields in unsupported nesting are rejected rather than silently omitted.

Each MP uses one transaction for their accepted declarations, spanning all their pages and parent
lookups. It commits before the next MP is fetched, and logs `Committed declarations for member …`.
Unchanged funding keeps its UUIDs,
including when entries are reordered. Any funding change replaces the whole group, preserving
multiplicity and the declaration UUID. Metadata-only changes update the header without replacing
funding. Every response is reparsed, even when its JSON is unchanged.

A parsing failure rejects the entire declaration before any write. Its previous metadata
and funding remain intact; a new rejected declaration creates nothing. Logs on stderr include its
source ID, member/request context, field path, original offending value and reason. Valid siblings
continue. A completed run exits **0** and prints an ordinary `status: succeeded` JSON summary with
member and accepted declaration counts, even if individual declarations were rejected.

Identical duplicate IDs within a run are logged and collapsed; conflicting duplicates fail the
refresh. Required HTTP failures after bounded retries, unreadable pages, database failures and
interruptions roll back only the current MP's writes; completed MPs remain committed and visible
to other connections. Fatal failures exit **1**, interruptions **130**.
Independent member refreshes remain committed. Missing declarations are left untouched; there is no
inferred withdrawal, deletion or missing-record state. Changed pagination totals are tolerated and
an empty page ends traversal. Execution remains externally controlled, without application locks.

## Validation and typed responses

`SearchPage.from_json()` and `HistoryBatch.from_json()` in the Parliament adapter validate
Parliament's JSON and translate response wrappers and source field names into typed objects.
The existing `exposed.models` imports remain available. Callers use
`page.members`, `page.total_results`, `page.skip` and `batch.histories`; they do not unpack raw JSON.
The response collections preserve source order. History batches retain duplicates so the importer
can detect them before indexing by ID.

Models ignore unused fields, reject wrong types for fields we use, and allow missing or null
optional fields. IDs are positive integers; booleans and numeric strings are rejected. Dates are
Python `date` values, serialized as `YYYY-MM-DD`. Source timestamps retain their calendar date:
time is discarded without timezone conversion. An absent end date remains null.

The importer requests pages of 100, advances by the number returned and stops at the API's
reported total. An empty page before that total fails the import so pagination cannot stall.
The core matches history batches to the requested member IDs; the PostgreSQL adapter owns
the atomic refresh transaction.

`Member.from_profile()` constructs a member and validates current Commons membership.
`CommonsService.from_history()` explicitly filters Commons memberships to the configured term,
then constructs dated `ServicePeriod` models. Their model validators check date ordering,
conflicting or overlapping periods, and agreement with current membership. Identical periods
are collapsed. These rules apply whenever the models are constructed, including outside the importer.

Validation errors include the model and field path without raw input values, and fail the entire
transaction. Database writes use explicit model attributes rather than depending on model field order.

## Tests

From the repository root:

```sh
make db-start
make check
# Or run tests that do not need PostgreSQL:
make test-unit
```

Integration tests create and drop their own randomly named `exposed_test_*` databases; the test role needs `CREATE DATABASE`. They do not modify the application database. The Makefile supplies the local test-server URL through `EXPOSED_TEST_ADMIN_DSN`. Direct pytest invocations without that variable explicitly skip integration tests. Test schemas are created by dbmate. All API responses in automated tests use synthetic fixtures; a live import is a separate end-to-end check.

See the [recorded verification results](../docs/verification/member-import.md) for automated and live import checks.

### Neovim and type checking

Ruff provides linting and formatting. Use Pyright alongside it for type checking; Ruff does not check whether values match their annotated types. The `[tool.pyright]` section in `pyproject.toml` selects `ingest/.venv` relative to that file and checks both `src` and `tests`. Pyright is included in the development dependencies and pinned in `requirements.lock`.

Astral also provides `ty`, a separate type checker and language server with Neovim support. This project currently verifies types with Pyright. See the [Ruff comparison](https://docs.astral.sh/ruff/faq/#how-does-ruff-compare-to-mypy-or-pyright-or-pyre) and [ty editor documentation](https://docs.astral.sh/ty/editors/).

## Implementation map

- `src/exposed/core/models.py` and `declarations.py`: validated domain values and rules.
- `src/exposed/core/refresh.py` and `refresh_declarations.py`: refresh use cases with injected ports.
- `src/exposed/core/ports.py` and `declaration_ports.py`: source and atomic storage contracts.
- `src/exposed/core/errors.py`: application-owned failures and safe diagnostics.
- `src/exposed/adapters/parliament.py`, `parliament_models.py`, `declarations.py` and `declaration_models.py`: HTTP, retries, pagination and source decoding.
- `src/exposed/adapters/postgres.py` and `declaration_postgres.py`: SQL, whole-refresh member transactions and per-MP declaration transactions.
- `src/exposed/composition.py`: client construction, connection lifetimes and production command runners.
- `src/exposed/cli.py`: configuration, argument parsing, JSON and exit codes through injected commands.
- The original top-level `api.py`, `models.py`, `db.py`, `importer.py` and `declaration_*.py` modules: compatibility facades.
- `../db/migrations/`: dbmate schema migrations.
- [Architecture and port contracts](../docs/architecture.md): dependency direction, consistency guarantees and test seams.
- [API research](../docs/research/uk-parliament-apis.md): source contracts and historical coverage.

Contains Parliamentary information licensed under the [Open Parliament Licence v3.0](https://www.parliament.uk/site-information/copyright-parliament/open-parliament-licence/).
