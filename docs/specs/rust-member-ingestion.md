# Rust member capture and PostgreSQL loading

## Problem Statement

Member ingestion currently runs in Python and combines live Parliament requests with database updates. Repeating the processing requires fetching the source again, and a later failure can leave earlier member batches committed. The Rust application has ingestion scaffolding, but its member request represents only one operation, its source and storage ports do not exchange records, and its data commands are not wired to an implementation.

The operator wants to capture member data once, inspect or reuse a versioned Parquet dataset, and explicitly choose when to apply that dataset to PostgreSQL. This first increment covers members and their Commons service only. Declaration ingestion and the wider cleaning and resolution pipeline are deferred.

## Solution

Provide two independent Rust operations. Fetch obtains all source observations needed to reproduce the existing member import and saves a complete, immutable Parquet capture identified by a UUIDv7. Load accepts that capture ID, validates its contents, and reconciles members and service periods in one PostgreSQL transaction without calling Parliament.

A failed fetch exposes no completed capture. A failed load commits no database changes. Reusing a capture does not require another API request. Repeated loads preserve existing identities, and an explicitly selected older capture is allowed to update the database.

## User Stories

1. As an operator, I want to fetch members through the Rust application, so that member acquisition no longer requires the Python command.
2. As an operator, I want database loading to be a separate command, so that fetching source data does not immediately change application data.
3. As an operator, I want to configure the Parliament term start, so that each capture has an explicit cohort boundary.
4. As a reader, I want everyone who served in the Commons during that term included, so that former MPs remain represented.
5. As a reader, I want former MPs whose latest House is the Lords retained, so that a later change of House does not erase their Commons service.
6. As an operator, I want one fixed observation date recorded per capture, so that later loading uses the original observation context.
7. As a reader, I want current Commons status derived from the current-members source observation, so that historical service and current membership remain distinct.
8. As a maintainer, I want the necessary member profiles and membership histories captured together, so that loading can run entirely offline from Parliament.
9. As an operator, I want fetching to work without a PostgreSQL connection, so that source acquisition is independent of database availability.
10. As a maintainer, I want typed Parquet datasets with an explicit schema, so that captured data can be inspected with ordinary data tools.
11. As a maintainer, I want raw captures to preserve the source fields used by the current importer, so that service rules can be reapplied without depending on a database export.
12. As a maintainer, I want schema versions recorded, so that future field additions can be distinguished from the first capture format.
13. As an operator, I want every fetch to receive a UUIDv7 capture ID, so that captures have unique identities with creation-time ordering.
14. As an operator, I want fetch to return its completed capture ID, so that I can pass that exact input to the load command.
15. As an operator, I want load to require an explicit capture ID, so that the filesystem does not choose my input implicitly.
16. As an operator, I want completed captures retained unchanged, so that another fetch cannot overwrite data used by a previous run.
17. As a maintainer, I want latest-capture ordering to consider only completed captures, so that a failed or unfinished fetch cannot become the latest usable input.
18. As an operator, I want a capture exposed only after every dataset is finalized, so that success never refers to partially written Parquet files.
19. As an operator, I want request, decoding, pagination, and storage failures to fail capture, so that incomplete source observations are not presented as a complete dataset.
20. As an operator, I want previous captures preserved after a failed fetch, so that existing inputs remain usable.
21. As a maintainer, I want identical repeated member profiles collapsed and conflicting repeats rejected, so that source duplication does not create ambiguous records.
22. As a maintainer, I want every requested member history accounted for exactly once, so that missing or mismatched histories cannot silently alter the cohort.
23. As an operator, I want bounded retries and useful source diagnostics, so that transient API failures are handled and persistent failures can be investigated.
24. As a maintainer, I want absent required member fields rejected during acquisition, so that ingestion never invents party or membership-location information.
25. As an operator, I want required database fields validated before writes, so that an invalid member produces a clear failure without partial database changes.
26. As a reader, I want source calendar dates interpreted consistently, so that loading on another day or in another timezone does not alter service dates.
27. As a reader, I want service periods clipped to the configured term using the existing rules, so that earlier service is not counted as service in the new term.
28. As a reader, I want departure and re-entry periods preserved, so that gaps in service remain visible.
29. As a maintainer, I want overlapping, contradictory, or otherwise invalid service periods rejected, so that stored service agrees with captured membership observations.
30. As an operator, I want members matched by Parliament's numeric ID, so that a changed name does not create another local member.
31. As a maintainer, I want existing member UUIDs preserved, so that application references and existing declarations retain their attribution.
32. As a maintainer, I want unchanged service periods to retain their UUIDs, so that repeat loads avoid identity churn.
33. As a reader, I want corrected service dates reconciled, so that stale intervals do not accumulate beside their replacements.
34. As an operator, I want previously stored members absent from a capture left untouched, so that source omission does not imply deletion or departure.
35. As an operator, I want the configured Parliament checked against the database, so that this increment does not silently perform a term rollover.
36. As a maintainer, I want a stored term end preserved, so that loading members does not erase information maintained elsewhere.
37. As an operator, I want all accepted member and service updates committed together, so that readers see one complete database refresh.
38. As an operator, I want any database failure to roll back the whole load, so that earlier members from that load are not left committed.
39. As an operator, I want repeated loading of the same capture to be safe, so that retries do not create duplicate members or service periods.
40. As an operator, I want to load an explicitly selected older capture, so that internal workflows can deliberately reapply saved data.
41. As an operator, I want load to use the capture's term and observation date, so that changing local configuration cannot reinterpret the saved cohort silently.
42. As an operator, I want capture identity and load outcomes reported, so that I can identify the input used and whether the operation succeeded.
43. As a maintainer, I want member fetch and load represented by typed service requests, so that their different inputs cannot be confused.
44. As a maintainer, I want source and storage ports to exchange typed records, so that the workflow does not rely on hidden shared state between adapters.
45. As a maintainer, I want the CLI to follow the existing verb-first data-command structure, so that the new operations fit the Rust application.
46. As a maintainer, I want one integration test that runs the real application through capture and loading, so that verification demonstrates that the complete workflow works.
47. As a maintainer, I want real Parquet compatibility and PostgreSQL persistence guarantees verified, so that successful orchestration also produces usable files and database records.
48. As an operator, I want existing server and declaration behavior preserved, so that this member-only increment can be delivered independently.

## Implementation Decisions

### Scope and application boundary

- Extend the existing Rust application and its ingestion service. This increment implements only member fetching and loading; it does not implement declaration stages.
- Preserve the separation between domain values and rules, ingestion orchestration, outbound source and storage adapters, and inbound CLI composition.
- The CLI parses arguments and configuration, constructs the required adapters, and submits a typed request. The service owns the workflow and returns a typed outcome.
- Use one canonical `ParliamentMember` domain model for capture, loading and search. Its party fields are required, its fields are private and exposed through methods, and its constructor rejects invalid values. Other domain values must also enforce their invariants at construction rather than through later validation calls.
- Retain EntityIngestionRequest and EntityIngestionTarget. The Members target gains a MemberIngestionStage with Fetch and Load operations. Fetch carries the configured term start. Load carries a CaptureId that validates a UUIDv7.
- Rename the member-source capability to describe the configured term cohort rather than only sitting members. Its contract must support current Commons observations, historical candidates, and membership histories.
- Replace unit-only source and storage contracts with typed inputs and outputs. A successful fetch outcome includes the finalized capture ID; a successful load outcome includes that ID and the saved term start and observation date. Do not return count summaries.
- Add the member refresh operation to the existing `ParliamentMemberRepo`, implemented by `ExposedDatabase`. Make the ingestion service interface accessible to the CLI. Infrastructure construction must not cause a database connection during Fetch or Parliament requests during Load.

### CLI and configuration

| Operation | Command hierarchy | Positional inputs | Result |
| --- | --- | --- | --- |
| Fetch | exposed, data, fetch, members | Configuration file | Completed capture UUIDv7 |
| Load | exposed, data, load, members | Configuration file, then capture ID | Capture ID, saved term start and observation date |

- Use Load for the Parquet-to-PostgreSQL operation. The existing declaration operation named Import already means acquisition into raw storage.
- Configuration supplies a local data root and the Parliament term start for Fetch. Load uses the data root and the PostgreSQL connection configuration, then reads the term start and observation date from the selected capture.
- Validate the settings required by the selected operation using the existing `TryFrom<&PathBuf>` configuration pattern. Preserve existing server configuration and behavior.
- Capture one Europe/London observation date at the beginning of Fetch. Reject a term start after that date. Loading later must not recompute the observation date from the current clock.
- Use the existing operational conventions for useful errors, progress logging, and machine-readable outcomes without count summaries. Include the capture ID in both success paths. Only report success after capture finalization or transaction commit, respectively.
- The initial Load command requires an explicit CaptureId. An implicit latest input and a convenience latest flag are outside this increment.

### Source acquisition

- Retrieve current Commons members through the current-members search and retain their IDs after validating duplicate profiles within that stream.
- Retrieve historical candidate profiles using the configured term start, fixed observation date, and the historical Commons membership filter. Do not apply a latest-House or current-member filter to this stream; a former MP may now have a Lords profile.
- Use historical candidate profiles as the profiles loaded into PostgreSQL. Current-search results supply current Commons status. Preserve the existing distinction between these two source observations.
- Fetch House membership histories for every distinct historical candidate. Returned history IDs must exactly match the requested IDs, without missing, extra, or duplicate histories.
- Follow Members API pagination using returned item counts and current response metadata. A premature empty page fails capture rather than stalling or silently truncating it. Use the API's documented search-page limit; the existing Python shared page size is not an API guarantee.
- Log and collapse identical repeated profiles within each search stream. Conflicting profiles for one source ID fail capture. Do not invent a cross-stream profile equality requirement.
- Verify that every current Commons ID appears among the historical candidates before finalizing the capture.
- Carry forward sequential requests, bounded timeouts, a short inter-request delay, and up to four attempts for transport failures, HTTP 429, and server errors. Preserve the existing bounded backoff and Retry-After handling. Persistent failures fail capture.
- Decode the source fields needed by the agreed raw schema. Structural or type errors that prevent representing those fields fail capture with member, field, and request context where available.

### Raw datasets and format

The first format is an explicit translation of the source information consumed by the current Python member importer. It is not a complete archive of every field returned by Parliament.

| Dataset | Row meaning | Required captured information |
| --- | --- | --- |
| Member profiles | One distinct historical candidate | Parliament member ID, display name, party ID and name, latest House, and latest membership location; all required |
| Current Commons observations | One distinct current Commons member | Parliament member ID |
| House memberships | One returned membership period for a candidate | Parliament member ID, House, source start date, and nullable source end date |

- Store these as three Parquet datasets with explicit types and nullability. Preserve source names and optional values without inventing replacements.
- Preserve all returned House membership periods needed for offline interpretation. Term-specific service clipping, removal of identical periods, and consistency checks belong to Load.
- Source date values retain their calendar date without timezone conversion, matching the existing importer. Store capture timestamps as UTC instants and the observation date as a calendar date.
- A manifest records the capture UUID, format/schema version, term start, observation date, capture start and completion timestamps, and the datasets and counts required to read and check that capture.
- Keep identifiers as source identities in raw data. PostgreSQL UUIDs and derived term-service rows are produced or reconciled during Load.
- Missing party or membership-location values fail Fetch before a capture is finalized; never substitute invented defaults. Readers can accept earlier schema-v1 files with nullable profile-column metadata only when all required values are present.
- Readers must reject missing datasets, incomplete captures, mismatched capture identity, and unsupported schema versions with useful errors.
- Future schema changes may add fields deliberately. Earlier captures cannot recover values that their schema did not store; automatic enrichment or rewriting of older captures is outside scope.

### Capture identity, versioning, and completion

- Generate a new UUIDv7 at the start of every fetch attempt. Each successful fetch creates a distinct immutable capture, including when its data is unchanged from a previous capture.
- Organize the filesystem by raw layer, members dataset, and capture UUID, in that order. Each capture owns its Parquet datasets and manifest.
- Build the capture in a staging location on the same filesystem. Finalize every Parquet writer and complete capture checks before exposing the final capture directory atomically.
- A request, decoding, pagination, validation, filesystem, or interruption failure must not expose a completed capture or return a successful capture ID. Previously completed captures remain intact.
- Normal failure handling should remove its staging artifacts where possible. Staging left by abrupt termination is not a completed capture and cannot be loaded or selected as latest. Resuming incomplete captures is outside scope.
- Load resolves the explicitly supplied UUID to its completed member capture and verifies the manifest. It does not guess an input from modification times or another stage's directory.
- Where latest-capture ordering is needed internally, it means the greatest UUIDv7 among completed member captures. It represents capture-ID creation order, not database load order.

### Offline validation and member rules

- Read the complete chosen capture and derive all member and service values before issuing database writes. Source access is unavailable to this operation by contract.
- Validate source identity, required database fields, capture completeness, and the existing member/service invariants. Fail the load with actionable member and field context when a value is invalid.
- Derive current Commons status from the captured current-ID set. A current member must have a latest Commons profile and qualifying Commons service.
- Retain Commons periods that start on or before the captured observation date and either have no end date or end strictly after the term start. An end date is the date service ceased; service ending on election day does not create service in the new term.
- Derive served-from as the later of source start and term start. Preserve the source end as served-until, including an absent end, rather than clipping it to the observation date.
- Collapse identical service periods and reject conflicting starts, overlapping periods, invalid date ordering, and disagreement between current status and service history. Preserve distinct departure and re-entry periods.
- Exclude a historical candidate with no qualifying Commons service, following current behavior. A capture producing no eligible members fails Load without writes.
- Store source calendar dates at midnight UTC in the existing timestamp columns, matching current persistence semantics.

### PostgreSQL loading

- Use the existing parliament_terms, members, and member_terms tables and their current constraints. This increment does not require a new schema or a capture-import registry.
- Begin one transaction for the entire validated member refresh. Check the stored term, reconcile profiles and service, and commit only after every write succeeds. Any failure or interruption rolls back all writes from that load.
- Preserve the single-configured-Parliament invariant. A different stored term start is an error; preserve any existing term end. Validate database-dependent invariants within the transaction.
- Upsert members by Parliament member ID and retain existing local UUIDs. Update their captured profile and current-status fields. Allocate UUIDv7 identities for newly created domain rows, following existing database conventions.
- Reconcile service using the existing member, term, House, and source-start identity. An unchanged period or corrected end retains its UUID. A corrected start replaces the old interval. Remove stale intervals only for the processed member and term.
- Leave previously stored members absent from the selected capture untouched. Preserve declarations and their member foreign keys; loading an older capture is an update of represented members, not a complete historical restoration of the database.
- Repeated loading of an unchanged capture must not duplicate members or service periods or change their identities.
- Permit loading any explicitly selected completed capture, including one older than a previous load. Do not introduce anti-downgrade checks, special approval flags, or persistent last-import bookkeeping.
- Report success only after committing the load. Do not calculate or return member-import count summaries.
- Execution remains externally controlled. Application scheduling and coordination of concurrent loads are outside this increment.

## Testing Decisions

### Single integration test

- Add exactly one integration test for this feature. Run the built Rust executable with real arguments and configuration through the member Fetch and Load commands.
- Use the production CLI, service, HTTP client, source decoding, domain rules, Parquet reader and writer, filesystem storage, PostgreSQL adapter, and transaction handling. Do not mock application components or add mocked unit-test or separate adapter-test suites for this feature.
- Use a temporary data root and an isolated temporary PostgreSQL database initialized from the real development baseline. Clean up these resources after the test.
- The only substituted external dependency is Parliament: a local HTTP fixture server supplies deterministic responses. The production HTTP client makes real requests to that server. No live Parliament access is required.
- Keep the test as one readable workflow with sequential phases. Assert returned IDs, command outcomes, actual file contents, and committed database state. Do not assert private helper structure or mocked call sequences.

### Workflow and assertions

1. Start the fixture server with a small paginated cohort containing current and former MPs and representative membership histories. Create the temporary data root and database, then invoke Fetch through the real CLI.
2. Verify that Fetch succeeds with a UUIDv7 capture ID, that the manifest and all three datasets are complete, and that a real Parquet reader sees the expected source values. Verify that fetching has not changed the database.
3. Stop the fixture server and invoke Load with that exact capture ID. Check the resulting term, member profiles, current/former status, and derived service periods in PostgreSQL. This establishes that loading uses captured data without Parliament access.
4. Load the same capture again. Compare database rows and UUIDs to establish that repeated loading does not create duplicates or change member and service identities.
5. Obtain a second capture with a profile or service correction. Arrange a real database rejection after earlier writes within the load transaction and verify that the failed load leaves the previous database state intact. Remove the test-only database rejection, load successfully, and check the correction and preserved identities. The first completed capture remains unchanged.
6. Make a subsequent Fetch encounter a source failure after receiving some data. Verify the failed command exposes no new completed capture and preserves the earlier captures. These failure phases belong to the same integration test.

Use the existing Python member integration scenarios as behavioral prior art and the existing SQLx-backed tests as examples of isolated database setup. Existing member and service rules remain implementation requirements; this task does not require a separate test case for every rule.

## Out of Scope

- Declaration acquisition, parent resolution, funding parsing, funder cleaning, entity resolution, and declaration or funding publication.
- Separate cleaned or resolved member layers, pipeline-wide dependency graphs, and delta calculation between captures.
- Raw JSON archives, automatic preservation of every unknown API field, and enrichment of old captures with newly introduced fields.
- Implicit latest loading, a latest CLI shortcut, incremental fetching, failed-capture resumption, and retention or garbage-collection automation.
- Historical-as-of API acquisition, multi-Parliament rollover, and changing the existing service-date semantics.
- Automatic loading immediately after fetch, scheduling, concurrent-import locking, remote object storage, and a general workflow engine.
- Database capture provenance tables, replay restrictions, application schema redesign, and changes to existing declarations or search behavior.
- Removal of the Python toolchain or migration of its declaration command. Rust member workflows must be usable independently without forcing that broader removal.

## Further Notes

- Implementation starts from the Rust ingestion scaffolding on feature branch feat/add-rust-cli-ingestion, reviewed at commit ef10839ca90605c3c55fd2c3489b060405579be0.
- [PR #13 review](https://github.com/johnnylarner/exposed/pull/13) refines the model, adapter and configuration conventions above. The review follow-up removes count summaries entirely; manifest row counts remain part of capture completeness validation.
- The existing Python member behavior is the reference for cohort selection, source interpretation, identity preservation, and service reconciliation. Source acquisition becomes a saved complete capture, database publication becomes atomic across the whole member load, and the reviewed model conventions reject incomplete member profiles during Fetch.
- A completed capture records observations obtained during a run. The inspected Parliament API contract does not guarantee a transactional snapshot across those requests; retain the existing consistency checks.
- Keep the repository's dedicated-worktree and linear-history practices. If implementation unexpectedly requires a database schema change, follow the repository's single reversible development-baseline policy and verify any live schema before reconciling migration metadata.
- API reference: [Parliament Members OpenAPI](https://members-api.parliament.uk/swagger/v1/swagger.json). Storage reference: [Apache Parquet file format](https://parquet.apache.org/docs/file-format/).
