# Ingestion architecture

## Migration inventory

The application refreshes one configured Parliament. This migration preserves the
schema, Python toolchain, sequential requests, existing public imports, and CLI
output and exit codes. The original member migration did not add term rollover or declaration ingestion;
the declaration migration is documented below.

| Operation | Existing path | Rules and effects to preserve | Compatibility evidence |
| --- | --- | --- | --- |
| Configure and run a refresh | `cli.main` → `importer.run_import` | Explicit working-directory `.env`, environment precedence, strict election date, fixed UK observation date, JSON summary and exit codes | `test_cli.py`, including production wiring with a real temporary database |
| Discover current and historical Commons members | `importer.search_pages` → `api.MembersAPI.search_page` | Pages of 100, current Commons filter, inclusive historical query dates, no latest-House filter on historical candidates, changing totals, failure on premature empty pages | `test_api.py`, `test_import_rules.py` |
| Match and validate histories | `importer.load_histories` → `api.MembersAPI.histories` → model constructors | Complete history batches, duplicate rejection, calendar dates, Commons service clipping, re-entry gaps, current-membership consistency | `test_models.py`, `test_import_rules.py` |
| Reconcile members and service | `importer._import_batches` → `db.ensure_term` / `db.write_member` | Single configured term, stable UUIDv7 IDs, profile outcomes, corrected intervals, absent members retained, stored term end preserved | `test_importer.py` with real temporary PostgreSQL databases |
| Complete or reject a refresh | `importer.run_import` | One transaction around all pages and final completeness checks; rollback on request, validation, write or interruption failure; safe diagnostic messages | Later-page and database-failure integration tests |

Baseline on 16 September 2026: `make check` passed with **103 tests**, Ruff lint
and formatting checks, and no Pyright errors or warnings. Dependencies were
installed from `ingest/requirements.lock` in the dedicated worktree.

## Ownership and dependency direction

| Module | Owns | Depends on |
| --- | --- | --- |
| `ingest/src/exposed/core/models.py` | Member values, Commons service invariants, single-Parliament invariant | Python and Pydantic |
| `ingest/src/exposed/core/refresh.py` | `refresh_members(term_start, as_of, source, store)`, history matching, cohort reconciliation and summary decisions | Core values, errors and ports |
| `ingest/src/exposed/core/ports.py` | `MemberSource`, `RefreshStore`, `MemberWriter` and profile write outcomes | Core values and Python protocols |
| `ingest/src/exposed/core/errors.py` | Expected validation failures, source/storage failures, import failure and safe diagnostics | Python and Pydantic |
| `ingest/src/exposed/adapters/parliament.py` | HTTP requests, retries, query parameters, pagination and the member-source implementation | Core contracts, HTTPX and response decoding |
| `ingest/src/exposed/adapters/parliament_models.py` | Parliament wrappers, source field aliases and calendar-date decoding | Core values and Pydantic |
| `ingest/src/exposed/adapters/postgres.py` | Connection factory, SQL, term rehydration, atomic refresh and driver-error translation | Core contracts and Psycopg |
| `ingest/src/exposed/cli.py` | Configuration, argument parsing, JSON output and exit codes through `run_cli` | Core errors and an injected command callable |
| `ingest/src/exposed/composition.py` | Concrete adapter construction, resource lifetimes and the fixed UK observation date | Adapters and the refresh interface |

Runtime flow is `cli.main` → composition's command → `refresh_members` → source
and storage ports. Source dependencies point into the core. The core has no HTTP,
SQL, connection configuration, environment lookup or clock reads. Pydantic remains
an ordinary validation/value dependency; its source aliases live at the HTTP edge.

The CLI's existing `main` function loads the composition command at startup, while
`run_cli` accepts a substituted command for parsing and output tests. Production
HTTP client settings and connection lifetimes live in `composition.py`.

## Port behavior

`MemberSource.current_commons()` streams profiles; `commons_candidates()` yields
bounded batches; `member_histories()` returns core histories. The HTTP adapter
retains the original pages of 100, query filters, changing-total handling, retry
limits and 1–100 history-request size rule. Typed decoding preserves order and
duplicates so the core can reject incomplete or mismatched history batches.

`RefreshStore.refresh(term_start)` yields a `MemberWriter` inside one atomic
scope. The PostgreSQL implementation uses a composition-owned connection from
`connect()` (autocommit enabled, with no surrounding transaction), and opens one
transaction covering term selection, every page and final validation. It commits
only after successful completion, and rolls back on request, validation, write
or interruption failure. The adapter invokes the core's single-Parliament
invariant when reading stored terms. Term ends and absent members remain intact.

The writer reconciles the profile and all service periods for one member, returns
`inserted`, `updated` or `unchanged` for the profile, and retains stable IDs.
Corrected source periods replace old intervals. No notifications, background jobs,
new transaction boundaries or concurrency guarantees were introduced.

Source and storage adapters translate dependency failures into `SourceError` and
`StorageError`, retaining original diagnostic causes. `ImportFailed` reports a
safe message and interruption status after rollback. The CLI preserves the JSON
schema and exit codes: success 0, import failure 1, invalid configuration 2 and
interruption 130.

## Compatibility facades

- `exposed.api`: `MembersAPI`, `BASE_URL` and `APIError` remain available.
- `exposed.models`: domain model and Parliament response imports remain available;
  existing `from_json()` entry points retain source parsing and validation paths.
  New core callers use `exposed.core.models` and Python dates directly.
- `exposed.db`: existing connection type and SQL helper imports remain available.
- `exposed.importer`: `run_import`, `connect`, `ImportFailed`, `safe_error`,
  `search_pages`, `load_histories` and `BATCH_SIZE` remain available. The importer
  delegates to the same core used by the CLI; it contains no separate refresh.
  Its legacy history helper retains request-size checks and raw parsing errors.
  Raw connection-opening errors remain compatible for `run_import` callers;
  composition translates them before they reach the CLI.

## Confirmed test seams and verification

The user confirmed all four seams on 16 September 2026: domain constructors,
refresh orchestration, source/atomic-storage ports and CLI behavior.

New interfaces were introduced through observed red-to-green cycles: core date
construction, infrastructure-independent refresh, source-failure rollback, the
HTTP source port, the PostgreSQL storage port and the injected CLI command.
Shared storage contract tests also caught and corrected a missing term invariant
in the in-memory implementation. Source-port tests caught the need to retain the
legacy history-request size guard. Existing behavior is covered by the original
characterization suite and additional CLI characterization checks.

- `make check`: **123 passed**, including **16 PostgreSQL integration tests**;
  Ruff lint/format checks passed and Pyright reported no errors or warnings.
  Temporary PostgreSQL databases were initialized by the real dbmate CLI.
- `make test-unit`: **107 passed, 16 deselected**.
- `ingest/.venv/bin/python -m pip check`: no broken requirements.
- `git diff --check`: passed.
- A core import audit permits only core modules, Python and Pydantic. A standalone
  refresh also succeeds with imports of HTTPX, Psycopg and dotenv explicitly
  blocked.
- The production CLI wiring check uses a substituted HTTP transport and real
  PostgreSQL, exercising argument parsing, composition, decoding, refresh and
  persistence together. It does not contact the live Parliament API.

All inventoried application paths reach the intended modules. SQL migrations and
the deployment/toolchain remain unchanged. Run the application from the repository
root with `make import-members`; see the importer README for configuration.

Pre-existing issue outside this migration: the CLI's setup error suggests
`make migrate`, but the current Makefile defines `db-migrate`. That existing error
text was retained for compatibility; this migration did not change Make targets.


## Declaration ingestion migration

The declaration feature now follows the architecture on `main` (`3d01433`). The
member refactor was merged first, retaining the separate declaration command and
shared HTTP retry implementation. Before changing the declaration boundaries,
all 32 declaration importer/CLI characterization tests passed. The merged member
API/CLI and declaration characterization checks then passed (50 tests).

### Scope and compatibility inventory

| Operation | Previous path | Boundary after migration | Preserved behavior |
| --- | --- | --- | --- |
| Configure and invoke the command | `cli.main` → `declaration_importer.run_import` | `cli.run_cli` → composition → `refresh_declarations` | Same arguments, configuration, JSON summaries, exit codes and independent member command |
| Retrieve and decode declarations | `declaration_api` and `declaration_models` | Parliament source adapter and response DTOs | All Commons categories/registers/dates, expired records, pagination, bounded retries, field diagnostics and raw payloads |
| Choose a published version | `SourceDeclaration.latest_fields` | Core `latest_version` policy over normalized dates/content | Newest register date, rejection of conflicting ties, no extraction from old versions |
| Complete declaration attribution | `Declaration.from_source` and importer parent resolution | Core draft acceptance and refresh orchestration | Ultimate-payer interpretation, required parent lookup, member identity, cycles, whole-declaration rejection and valid siblings |
| Read cohort and publish | Direct SQL in `declaration_importer` / `declaration_db` | Atomic declaration-storage port and PostgreSQL adapter | Distinct stored members, stable UUIDs, multiplicity, whole funding-group replacement, no inferred deletion, one refresh transaction |
| Detect duplicate delivery | Importer raw dictionaries | Core evidence comparison before source interpretation | Identical content logged/collapsed; conflicts fail and roll back the refresh |

### Declaration boundary map

| Module | Owns |
| --- | --- |
| `core/declarations.py` | Validated declaration/funding values, funder precedence, latest-version selection, parent completion, transient source records and funding equality |
| `core/refresh_declarations.py` | `refresh_declarations(term_start, source, store)`, duplicate detection, parent resolution, rejection logging, acceptance and refresh results |
| `core/declaration_ports.py` | `DeclarationSource`, `DeclarationStore`, `DeclarationWriter` and their failure/atomicity contracts |
| `adapters/declarations.py` | Interests HTTP requests, pagination, timestamps, source context and exception translation |
| `adapters/declaration_models.py` | Source aliases, nested Parliament fields, exact decimal parsing and translation into core drafts |
| `adapters/declaration_postgres.py` | Cohort SQL, row rehydration, UUID preservation, funding replacement, PostgreSQL transactions and driver-error translation |
| `composition.py` | Concrete clients/connections and both production command runners |
| `cli.py` | CLI parsing and output, with separate injected member and declaration command callables |

`DeclarationSource.declarations()` yields bounded batches of `RetrievedDeclaration`.
Raw responses are held in memory during a refresh for exact duplicate comparison,
parsing and parent resolution. They are never persisted. The core treats each
payload as opaque: it never navigates Parliament keys, field arrays or response
aliases. The adapter supplies the identifier, retrieval time and diagnostic context.

The core checks duplicate evidence **before** calling `DeclarationSource.interpret()`.
That method translates the source shape into a `DeclarationDraft` or raises the
core-owned `DeclarationParseError`. This split preserves duplicate-before-parsing
behavior and permits a single bad item to be rejected without losing its page.
The adapter uses the core version policy before decoding that version's fields;
malformed historical funding does not invalidate a valid latest version. Only
parsed declaration fields, funding and retrieval times reach the storage port.

The adapter decodes normalized funder candidates and delegates their precedence to
core `preferred_funder()`. A candidate's decoding error is propagated only when
that name is needed, preserving the treatment of malformed unused fallbacks.

`DeclarationDraft.accept()` produces a complete `Declaration`. A draft that needs
its parent's payer cannot be accepted until that parent has been resolved. An
explicitly different, withheld ultimate payer stays absent. The writer receives
the accepted `Declaration` and its retrieval time directly; there is no snapshot wrapper.

`DeclarationStore.refresh(term_start)` yields a writer inside one atomic scope.
The writer reads the existing cohort and stages parsed declarations and funding. PostgreSQL commits only
when the core exits successfully, and rolls back on source, storage, validation or
interruption failures. Individual declaration rejections are caught before writes,
logged and skipped, preserving the existing completed-success exit behavior.
The declaration schema stores parsed fields only. Locking, scheduling, retry and
member-refresh semantics are unchanged.

### Compatibility and verification

The original `declaration_importer.run_import(database_url, term_start, api)` now
wires the same core through composition. `declaration_api` and `declaration_models`
retain source-client/model imports; `declaration_db` forwards parsed values and retrieval time
to the PostgreSQL adapter, with no source-payload argument.
Legacy response-shaped declaration construction delegates to core acceptance.
These facades do not contain parallel refresh or acceptance implementations.

Tests retain the full existing importer/CLI characterization coverage with real
PostgreSQL and add domain policy tests, infrastructure-free refresh tests, shared
memory/PostgreSQL atomic-store tests, source-port error tests, and substituted CLI
command tests. The production CLI tests still exercise real wiring with a mock
HTTP transport and temporary databases initialized by dbmate.

`test_architecture.py` audits both cores' import direction and runs a declaration
refresh in a fresh Python process with HTTPX, Psycopg and dotenv imports blocked.
Run `make check` for formatting, lint, type checking and the full suite;
`make test-unit` runs without PostgreSQL. Use `make import-declarations` for the
existing declaration command after applying its existing schema migrations.

Final declaration-migration verification on 16 September 2026: `make check`
passed with **177 tests**, including real PostgreSQL rollback/isolation checks,
Ruff lint and formatting, and no Pyright errors or warnings. `pip check` found no
broken requirements. Both standards and spec reviews finished with no outstanding
findings; attribution precedence was moved into the core during review.
