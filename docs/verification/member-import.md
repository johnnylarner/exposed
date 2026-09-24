# Member importer verification

For the current migration tooling, see the [SQLx verification](#sqlx-migration-tooling)
and [migration instructions](../../db/README.md). Earlier sections record historical checks.

Verified on **15 September 2026**, using Python 3.14.6 and the isolated Compose PostgreSQL 17.11 database.

## Automated checks

- Full suite after switching to dbmate: **34 passed**, with `EXPOSED_TEST_ADMIN_DSN` set; the PostgreSQL integration tests ran.
- Pyright 1.1.414: **0 errors, 0 warnings** across `src` and `tests` in standard mode.
- Ruff lint and formatting checks passed.
- `pip check` reported no broken dependencies.
- SQL migrations now live in `db/migrations/` and are applied by dbmate rather than packaged with the importer.

The suite covers page iteration, missing/duplicate results, request retries, required histories, service gaps, original dates, late entrants, former MPs whose latest House is Lords, changing profiles, repeat imports, nullable term ends and removal of audit tables. The importer assumes one update at a time.

The later-page failure test reads the import connection to confirm the first batch has already been written, and reads through a second connection to confirm those changes are still uncommitted. It then fails the next API page and verifies that all member, term and service data return to their previous values. Another test forces a PostgreSQL constraint failure after the first batch and verifies the same rollback guarantee. Final completeness validation is also checked after writes have begun.

## Initial live imports

These two runs preceded the simplification and audit removal. The response retention and run IDs below describe the old implementation. Both fetched the Members API and committed successfully to local Postgres.

| Result | First import | Repeat import |
| --- | ---: | ---: |
| Members who served | 655 | 655 |
| Current Commons members | 649 | 649 |
| Former Commons members | 6 | 6 |
| Service periods | 656 | 656 |
| Inserted member profiles | 655 | 0 |
| Changed member profiles | 0 | 0 |
| Unchanged member profiles | 0 | 655 |
| Raw responses retained for this run | 99 | 99 |
| Excluded historical candidates | 0 | 0 |

Run IDs:

- First: `01a0a549-90f7-757d-96d8-251432aa85fb`
- Repeat: `01a0a54d-4be3-754e-8492-0cbb146e4ee2`

The member, Parliament term and service ID sets were identical after the repeat import. The database retained **198** raw responses across those two successful runs. The [verification queries](../../ingest/scripts/verify.sql) found zero invalid service dates and zero duplicate Parliament member IDs.

The live data includes one member with two service periods separated by a gap; both periods are retained. These totals describe this observation date and are not hard-coded expectations for future imports.

The local Postgres instance is available on `127.0.0.1:55432`. Setup and hosted-database configuration are documented in the [README](../../ingest/README.md).

## Audit removal and term end migration

Applied `002_remove_audit_add_term_end.sql` to the local database on 15 September 2026. Compared every non-audit field before and after within the migration transaction: all **655 members**, **656 service records** and **1 Parliament term** were preserved. Confirmed that `term_end` is a nullable date and both audit tables are absent. Verification queries returned zero invalid service dates and zero duplicate member IDs.

The updated implementation passed all **33 tests**, including PostgreSQL integration tests, plus Ruff lint/format checks and Pyright.

## Live refresh after transaction simplification

On 15 September 2026, ran the simplified importer against the live Members API and the existing local database, with migration `002` already applied. It wrote the batches inside one transaction and committed successfully: **655 members**, comprising **649 current** and **6 former** Commons members, with **656 service periods**. All 655 profiles were unchanged; no profiles were inserted or updated.

Compared IDs before and after the refresh: member IDs and their Parliament member ID mappings, service IDs and the Parliament term ID were unchanged. The verification queries returned **zero invalid service dates** and **zero duplicate member IDs**. The final source and test files also passed all **33 tests**, Ruff lint/format checks and Pyright with **0 errors and 0 warnings**.

## dbmate development baseline

Replaced the Python migration runner and the two old migrations with `db/migrations/20260915000000_initial.sql`. No legacy upgrade mechanism is retained.

Verified with dbmate **2.35.1**:

- `make check`: **34 passed**, Ruff checks passed, Pyright reported no errors or warnings.
- Fresh test databases are initialized by the real dbmate CLI.
- Repeating migrations applies nothing; reversing and reapplying the baseline succeeds in an empty disposable database.
- `make migrate` and `make migration-status` succeed locally, reporting one applied migration and zero pending.
- `make migration-new` generated the expected up/down template; the temporary file was removed.
- Switched the local development database's tracking table to dbmate and compared all member, service and term records before and after; domain data was unchanged.

## SQLx migration tooling

Verified on **22 September 2026**, using SQLx CLI **0.9.0** and the local Compose
PostgreSQL database. The single SQLx baseline includes the existing `pg_trgm`
extension and member/funding `NOT NULL` constraints.

- Before the refactor, `make check` with dbmate produced **186 passed, 20 failed**.
  After switching to SQLx it produced **190 passed, 20 failed**, with exactly the
  same failing test names. The four new migration checks all passed. Existing
  failures come from importer/test expectations of nullable fields conflicting
  with the already committed `NOT NULL` migrations; this tooling change preserves
  those constraints.
- Ruff lint/format checks and Pyright passed with no errors or warnings.
- Fresh migration, repeat migration, status, and database reset commands passed.
  Reset was exercised only against a disposable database. Tests verified checksum
  mismatch rejection and existing-schema adoption without losing records.
- Fresh and live `exposed` schema dumps matched after excluding pg_dump's random
  restriction tokens. Extension versions and schemas matched too, including
  `pg_trgm` in `exposed`.
- Adopted the verified live schema with `sqlx migrate override skip` and confirmed
  `make db-migrate` succeeds with one installed baseline. The previous dbmate
  history remains as unused metadata.
- Before/after row counts and full-row fingerprints matched for all five domain
  tables: **1 Parliament term, 655 members, 656 service records, 10,973 declarations,
  and 8,620 funding entries**. The live schema was checked again after adoption.

## Member batch commits

Verified on **24 September 2026**, using synthetic API responses and disposable
PostgreSQL databases initialized by SQLx. Member ingestion now commits each candidate
batch before fetching the next page; this supersedes the whole-refresh transaction
behavior recorded above.

- **138 member-ingestion and related tests passed**, including PostgreSQL checks
  that observe committed profiles and service through an independent connection
  before the next page is fetched. A database failure after a write in the second
  batch rolls back that batch while preserving the first. Source failures,
  interruptions, conflicting duplicates, final completeness failures and retries
  are covered as well.
- Ruff lint/format checks passed; Pyright reported **0 errors and 0 warnings**.
- The shared `database_url` test fixture was missing on `main`, causing **62 setup
  errors**. Restoring SQLx initialization allows the integration suite to run.
  Member storage fixtures now supply the metadata required by the baseline schema.
- Full `make check`: **196 passed, 18 failed**. All 18 failures are in declaration
  ingestion and also reproduce on `main` with the database fixture restored.
  That baseline run produced **188 passed, 20 failed**; the other two failures were
  the member storage fixtures corrected here. No new failing tests were introduced.
- No live import or application database migration was performed.
