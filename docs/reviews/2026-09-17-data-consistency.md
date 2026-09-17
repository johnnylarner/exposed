# Code review and data consistency audit

Whole-codebase review on 17 September 2026 at `e5fed68`, with follow-up decisions
and changes recorded below. The review covered member and declaration ingestion,
models, adapters, configuration, migrations, tests, and documentation.

The local data audit used read-only SQL. It checked all 171 declarations with
missing registration dates and all 223 ongoing-payment declarations against the
official API, plus 14 source examples. It did not refresh application data or
exhaustively compare every stored declaration with the live service. Counts below
are observations from that review, not assertions about future imports.

## Follow-up decisions and changes

- Development schema compatibility is not required. All tables and current fields
  now live in [one baseline](../../db/migrations/20260915000000_initial.sql).
  Recreate development databases after schema changes and ingest the data again;
  an upgrade path from previous development schemas is intentionally not maintained.
- New ingestion populates registration dates when supplied by Parliament. The
  [database instructions](../../db/README.md) and [importer documentation](../../ingest/README.md)
  describe that workflow. Missing source dates remain null.
- Duplicate member profiles are validated independently within the current and
  historical source streams. Identical repeats are logged and counted once;
  conflicting repeats fail and roll back the member refresh. The
  [importer regression tests](../../ingest/tests/test_importer.py) cover both
  within-page and across-page delivery, stable IDs, accurate counts, and rollback.
- [Setup instructions](../../README.md) now include dependency installation,
  environment configuration, and the existing `make db-migrate` target.
- Investigate [payment periods](../notes/payment-periods.md) and
  [funder names](../notes/funder-names.md) later. Their notes preserve the audit
  examples and open questions without changing ingestion behavior.
- Currency conversion and payment-context modeling are deferred; no changes were
  made to those fields or their interpretation.

## Observed data

| Check | Result |
| --- | --- |
| Stored members | 655 |
| Declarations | 10,973, across 649 members |
| Funding entries | 8,620 |
| Source currency labels | All 8,620 are `GBP` |
| Payment types | 5,287 `Monetary`; 3,333 `In kind` |
| Missing funding amounts, currencies, or funders | 0 |
| Negative amounts / amounts with fractional pence | 0 / 0 |
| Zero amounts | 2; both source-supplied |
| Duplicate source declaration IDs / orphan relationships | 0 / 0 |
| Identical funding rows within a declaration | 0; legitimate multiplicity must still remain supported |
| Category IDs with conflicting stored labels | 0 |
| Invalid stored service dates / current-membership mismatches | 0 / 0 |
| Missing registration dates | 171; all 171 also null in the latest selected source version |
| Funder groups with case/whitespace variants | 62; these are candidate text groups, not verified shared identities |

Six stored members have no declarations. This observation alone does not establish
an ingestion defect or justify fabricating records. The audit did not verify their
absence individually against the source.

## Verification

The eight duplicate-member regression cases reproduced the missing validation
before the fix and passed afterward using synthetic HTTP responses and disposable
PostgreSQL databases. The complete `make check` then passed **191 tests**, Ruff
lint and formatting, and Pyright with zero errors or warnings.

The consolidated migration was also applied, applied again, rolled back, and
reapplied to a disposable database. Its five tables and current column definitions
were verified, including nullable registration dates. Documentation links and
`git diff --check` passed. Application data was not changed.
