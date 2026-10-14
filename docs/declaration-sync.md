# Declaration synchronization

Daily synchronization reads stored Parliament member IDs as ownership prerequisites. It does not fetch member lists, load members, or refresh member metadata. Maintain members separately.

Initialize complete declaration evidence explicitly before running daily synchronization.

```sh
export DATABASE_URL='postgresql://...'
exposed data declarations reconcile --config declaration-sync.yaml
exposed data declarations sync --config declaration-sync.yaml
```

Example configuration.

```yaml
data_dir: /absolute/durable/path/declaration-evidence
batch_size: 100
candidate_budget: 1000000
overlap_days: 7
indexing_lag_days: 1
# Optional inclusive UTC source date. Otherwise UTC today minus indexing_lag_days.
# cutoff: 2026-10-13
# Production default. Override only for a controlled source or local tests.
interests_url: https://interests-api.parliament.uk/api/v2/Interests
```

The database selects the committed capture manifest, revision, and inclusive discovery checkpoint. Daily acquisition unions published-date and updated-date discoveries, then hydrates each in-scope ID without date filters. Filtered discovery payloads can omit older versions and never replace full evidence. Complete records replace their previous funding-entry sets. Unchanged declarations retain their actual acquisition timestamps.

The initial reconciliation captures every stored member, including members with zero declarations. When preexisting database declarations are absent from the initial capture, initialization fails. The current projection cannot reconstruct complete raw source evidence. Resolve the discrepancy before initializing the baseline. Later reconciliations retain previously captured declarations omitted by successful source responses and report the omission count. A source omission does not automatically delete a declaration.

Daily scope remains pinned to the committed member-ID cohort. Newly stored members appear as pending reconciliation, and their discoveries appear as exclusions. Explicit reconciliation captures their full declaration histories before adopting them into scope. A known declaration changing ownership is an error. Missing or malformed registrant information also fails acquisition. Missing parent IDs receive unfiltered context requests. Genuine missing or cross-member parents retain the existing unavailable-attribution behavior.

Each sealed run contains `capture.json`, `capture.sha256`, and hashes for all member Parquet partitions. The manifest records the scope, source policy, date bounds, expected publication revision, complete source records, retrieval instants, discovery page receipts, exclusions, and omissions. Capture stages a new directory, flushes its files and directories, then atomically renames it. Previous snapshots remain unchanged. Retain the artifact root and every committed run. Do not select a base using the newest filesystem directory.

If interpretation or publication fails after sealing, the error names its run ID. Continue the same evidence with the same configuration path.

```sh
exposed data declarations sync --config declaration-sync.yaml --resume RUN_ID
```

Resume performs no source requests. It validates sealed raw evidence and completed derived-stage hashes. If a crash leaves a derived stage without its durable hash receipt, resume discards that unsealed derived output and recomputes it from sealed raw evidence. It never changes the captured source evidence. A failed capture before sealing cannot resume and must be reacquired.

Publication holds the existing declaration loader advisory lock. Exact retries return before checking their old expected revision. A competing prepared run with a stale revision fails without changing declarations, funders, grouping, or coverage. Projection replacement and snapshot/checkpoint selection commit in one transaction. Legacy `declarations load` refuses to modify a projection once synchronization owns it. Empty and content-equal runs commit coverage and revision without rerunning resolution or replacing funders.

Date overlap and periodic reconciliation mitigate source omissions. Offset pagination and date filtering do not provide snapshot isolation or a lossless changefeed. The adapter rejects HTTP failures, duplicate IDs, inconsistent totals, and premature empty pages, but cannot detect every concurrent source mutation. Reconcile periodically and inspect omission and exclusion reports.

Interpretation still cleans, resolves, and groups the complete corpus whenever source content changes. No performance improvement has been measured. Noncompany funder identities remain scoped to interpretation runs, so funder IDs and URLs can change after a content update. Automatic declaration retirement and stable funder IDs are separate policies.

## Verification

The integration test starts a deterministic local Interests HTTP server, invokes the real CLI binary, and uses SQLx to create an isolated migrated database. `DATABASE_URL` must identify a disposable development server on which the role can create databases. The test does not modify the database named in that URL.

```sh
DATABASE_URL='postgresql://.../development?sslmode=disable' \
  cargo test -p exposed --test declaration_sync -- --nocapture
```

The test compares baseline-plus-delta publication with complete final evidence, including funding attribution, amounts, aliases, and funder grouping. Fixed assertions check corrected parent attribution, removed funding entries, retained historical aliases, and unavailable parents. It exercises distinct published-only and updated-only discoveries, unfiltered version hydration, preserved timestamps, checkpoint overlap, rollback on source and paging failures, empty/content-equal runs, exact replay, stale publication, and sealed-run recovery.
