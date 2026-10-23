---
name: verify
description: Compare selected Exposed searches and authoritative MP and organisation totals with live production, using an isolated snapshot-backed Compose build of the current worktree. Use after search, grouping, totals, or public loader changes.
---

# Verify against live production

Read [the feature map](features/README.md). From the repository root:

```sh
npm ci --prefix .agents/skills/verify
npm test --prefix .agents/skills/verify
./.agents/skills/verify/scripts/run-snapshot-comparison.sh
```

For totals or ranking changes, also run the
[current-term checks](features/current-term-totals.md). Production parity alone
does not prove that totals exclude earlier and undated records. Report the
current-term result separately from the live comparison result.

Require Docker Compose, Node with built-in fetch/test, curl, and an existing
`exposed_search_regression` snapshot in container `exposed-postgres-1` (user
`exposed`). Override those prerequisite identities with `VERIFY_SNAPSHOT_CONTAINER`,
`VERIFY_SNAPSHOT_DATABASE`, and `VERIFY_SNAPSHOT_USER` when necessary. The helper
can start a stopped source container and restores that initial state; it never
stops a source container that was already running. Do not run during snapshot
refresh. A missing snapshot is a failed prerequisite, never permission to ingest.
The [refresh helper](scripts/refresh-search-regression-db.sh) remains a separate,
deliberate replacement operation requiring its documented source-client shutdown.

The helper creates a unique project for this worktree/run, dumps one consistent
source snapshot, builds this worktree's backend/frontend with the production
Dockerfiles, and restores into its own PostgreSQL volume. Only that disposable
copy receives current migrations; checksum mismatches fail without alignment.
It does not rerun ingestion or refresh stored groups. The backend uses a read-only
connection; the frontend reaches its private API within the project's network.
No shared target/cache volumes or database ports are mounted. A random loopback
frontend port supplies the comparator's development origin. Doctor checks backend
health and decodes a successful public search model before comparison.

Production defaults to `https://exposed.today`. Every invocation reads it live
with bounded sequential GETs through the same SvelteKit page-data loaders used by
the public frontend. [The manifest](live-cases.json) contains selectors only,
never expected money. Resolve funder UUIDs independently in each environment by
one exact name/kind match, then assert the identity returned by each detail page.
Member Parliament IDs and names must match. Missing or ambiguous identities,
error models, malformed fields, redirects, HTTP failures and timeouts fail closed.

Compare supplied decimal strings/null without arithmetic, rounding, currency
conversion, or fallback totals. Search preserves order, duplicate multiplicity,
match provenance and entity context. Totals include authoritative declared periods,
counts/completeness, currencies/categories, parties and ranked recipients. Arrays
retain the public loader's order. Installation-local funder UUIDs are omitted only
from semantic comparisons; raw responses retain them.

The printed new evidence directory survives success, mismatch, failure and cleanup.
It records the worktree commit/dirty status, source container and snapshot, unique
Compose project, derived origin, build/migration/doctor/container/cleanup logs,
raw request URLs/status/headers/body/timestamps, normalized observations, incremental
`http/report.json`, and readable `http/diff.txt`. An interrupted comparator leaves
an incomplete report and available partial evidence. Treat incomplete evidence as
failure. Evidence retains the database dump.

Exit 0 requires every observation to succeed and match. Exit 1 means observed field
differences; exit 2 means observation/configuration/schema/identity errors (which
take precedence over differences). Report the code and evidence paths. Snapshot
age or live production changes can explain discrepancies, but discrepancies still
fail; investigate provenance before attributing them to branch code. Never edit
expected totals or refresh data to manufacture parity. There is no baseline-update
mode. If the snapshot predates grouped funder metadata, missing group identities
can be valid observed errors.

Cleanup always removes only this run's Compose project and volumes, including on
failure. It retains the source snapshot and filesystem evidence. Check
`cleanup.log`; failed cleanup is exit 2. Do not use blanket Docker prune/down.

For controlled HTTP replay or diagnosis, the internal observer accepts explicit
origins; this alone does not prove a worktree build:

```sh
node .agents/skills/verify/scripts/compare-live.mjs --development http://127.0.0.1:PORT
```

Optional `--production ORIGIN`, `--cases PATH`, and `--evidence NEW_DIRECTORY` select
a controlled reference, selector subset, or evidence location. Equal origins are
rejected. The lifecycle command accepts only `--production` and `--cases`; it always
derives its own development origin and evidence directory. No production writes,
CI changes or financial-semantic changes are part of verification.
