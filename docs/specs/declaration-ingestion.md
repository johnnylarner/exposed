# Declaration ingestion

## Problem Statement

Exposed currently imports members and their Commons service into PostgreSQL, but it cannot yet import the financial declarations needed for later display. The original donation sketch assumes a donor and an amount per record. Parliament's declarations are broader: they include nonfinancial disclosures, declarations with several funders, in-kind benefits, and payments whose payer information belongs to a related declaration.

The project needs a simple model that links declarations to their source IDs, exposes useful funding fields, and extends the current Pydantic-based importer without taking ownership of member updates. Repeated imports must update existing declarations consistently. A malformed amount must be diagnosable through logs and must not leave a declaration with only some of its funding imported.

## Solution

Add a separate import-declarations command. It reads the existing member cohort from the database and retrieves the available Commons declarations for those members, across all categories and available dates, including expired declarations.

Store one declaration record per source declaration ID, linked directly to the existing member, with its readable category and retrieval time. Persist only parsed fields; do not store raw source JSON. Store zero or more funding entries separately, each with its own UUID and the source declaration ID that groups it with its declaration.

Funding entries expose the source funder, exact decimal amount, currency, and payment type. Prefer an explicitly supplied ultimate payer; otherwise use the source donor or payer, resolving the parent declaration when required. Store the selected name without retaining alternative payer identities or attempting to identify shared donors across declarations.

Parse the whole declaration before changing its stored representation. If parsing fails, log the failure, keep the previously stored declaration and funding unchanged, and continue. A new rejected declaration produces no records. Publish all accepted changes together. An otherwise completed run succeeds even when individual declarations were rejected and logged.

## User Stories

1. As an operator, I want a separate declaration import command, so that I can refresh declarations independently of member ingestion.
2. As an operator, I want the command to read the existing database cohort, so that declaration ingestion does not need to refresh or reconstruct members.
3. As a reader, I want declarations linked to the member who made them, so that corrected service dates do not change their attribution.
4. As a reader, I want declarations for everyone who served in the configured Parliament, including former MPs, so that the dataset reflects the existing cohort.
5. As a reader, I want all Commons declaration categories retained, so that disclosures beyond donations remain available for display.
6. As a reader, I want human-readable category names, so that I can understand a declaration without interpreting a numeric category ID.
7. As a reader, I want older and expired declarations included when the API supplies them, so that the configured term does not unnecessarily truncate a member's available history.
8. As a reader, I want coverage described as available API history, so that I am not led to assume that a complete lifetime archive has been collected.
9. As a maintainer, I want only parsed declaration and funding fields persisted, so that the database model stays simple.
10. As a maintainer, I want unused source fields discarded after processing, so that this feature does not become an archive of API responses.
11. As a reader, I want the declaration's successful retrieval time available, so that I can understand the age of the stored data.
12. As a maintainer, I want a stable local declaration UUID and a unique source declaration ID, so that repeated imports update the same declaration.
13. As a reader, I want several funding entries to be grouped by their API declaration ID, so that a declaration with several funders remains understandable.
14. As a reader, I want each amount paired with its actual source funder, so that nested donor groups do not become unrelated lists of names and values.
15. As a reader, I want nonfinancial declarations retained without fabricated funding entries, so that an unpaid role is still represented accurately.
16. As a reader, I want decimal amounts and source currency codes preserved, so that ingestion does not introduce rounding or currency assumptions.
17. As a reader, I want payment type retained, so that a monetary payment can be distinguished from the stated value of an in-kind benefit.
18. As a reader, I want an explicitly stated ultimate payer used as the funder, so that the most specific source attribution is available directly.
19. As a reader, I want parent-declaration payer details resolved when necessary, so that a child payment remains attributable even when its own payload omits the payer's name.
20. As a maintainer, I want source names retained without cross-declaration identity matching, so that similar names do not silently merge unrelated funders.
21. As a reader, I want current funding derived from one applicable source version, so that successive published versions are not counted as separate funding entries.
22. As a maintainer, I want returned versions used only to select and parse the latest applicable content, so that historical versions do not need their own storage.
23. As an operator, I want unchanged funding entries to retain their UUIDs on a repeated import, so that reruns do not create identity churn.
24. As an operator, I want changed funding replaced as a complete group, so that removed or corrected entries cannot accumulate alongside their replacements.
25. As a reader, I want legitimate repeated entries within one declaration preserved, so that two equal contributions are not collapsed merely because their values match.
26. As an operator, I want metadata-only source changes to preserve unchanged funding UUIDs, so that source updates do not unnecessarily recreate funding rows.
27. As a reader, I want each accepted declaration's metadata and funding published together, so that the stored fields describe the same accepted update.
28. As an operator, I want any partial parsing failure to reject the entire declaration, so that a multi-entry declaration is never silently imported in part.
29. As a reader, I want a rejected update to preserve the previous complete declaration and funding, so that a malformed correction does not replace usable data with an inconsistent mixture.
30. As an operator, I want a rejected new declaration to create no partial records, so that incomplete ingestion is not presented as a valid declaration.
31. As a maintainer, I want parsing logs to identify the source declaration, field path, offending value, and reason, so that I can extend parsers for previously unsupported numbers.
32. As an operator, I want valid declarations on the same page as an invalid declaration to continue, so that one bad item does not discard its valid siblings.
33. As an operator, I want per-declaration parsing failures handled through logs, so that a completed run does not require a special partial-success exit status.
34. As an operator, I want a failed source request to roll back accepted writes from that refresh, so that an interrupted fetch does not publish an unfinished run.
35. As an operator, I want a database failure to roll back the refresh, so that earlier writes are not left committed after a later write fails.
36. As an operator, I want identical repeated declaration IDs within a run reported and collapsed, so that duplicate source delivery does not duplicate funding.
37. As an operator, I want conflicting content under one source ID to fail the refresh, so that ingestion does not arbitrarily choose between conflicting responses.
38. As an operator, I want an ID already stored by a previous run treated as an update, so that normal refreshes are not mistaken for duplicate-source failures.
39. As an operator, I want changing pagination totals tolerated, so that ingestion does not enforce a missing-record policy that this feature does not require.
40. As an operator, I want existing declarations absent from a response left untouched, so that omission does not become an inferred withdrawal or deletion.
41. As a maintainer, I want source data reparsed on later refreshes, so that improved parsing can produce corrected funding even when the source payload itself has not changed.
42. As a maintainer, I want declaration validation to follow the existing Pydantic model approach, so that the extension remains consistent with the current refactor.
43. As an operator, I want member records and member-import behavior preserved, so that this feature remains a consumer of the existing member model.
44. As an operator, I want execution to remain externally controlled, so that this tool does not introduce application locking or scheduling mechanisms.
45. As a maintainer, I want deterministic tests through the importer with synthetic API responses and real PostgreSQL, so that data guarantees can be checked without relying on a changing live service.

## Implementation Decisions

### Scope and ownership

- The new command is import-declarations. It uses the existing database configuration and the cohort represented by members, parliament terms, and member service periods.
- Select distinct members who served in the configured stored Parliament. Multiple service periods must not cause duplicate retrieval for the same member. Current Commons status is not the cohort filter.
- Member ingestion remains the owner of member and service data. Declaration ingestion reads that data and does not create, refresh, or reconcile members or terms.
- A declaration belongs directly to the stable member record. It does not belong to a replaceable member-service-period row.
- The term restricts which people are queried. It does not introduce a declaration publication, registration, or event-date cutoff.

### Source retrieval

- Use the Register of Interests API v2 with the Commons register type explicitly selected. Include all categories and expired declarations, and search all available registers rather than selecting only the current publication.
- Retrieve declarations for the stored member IDs, following pagination and resolving source relationships when needed for attribution. Prefer separate source items for child declarations so each source ID has one logical imported declaration.
- Follow pagination using returned items and the current response metadata. Tolerate changed totals and ensure traversal terminates rather than repeating an empty page indefinitely. Do not introduce reconciliation against an expected complete set of declarations.
- Refresh from the relevant source declarations rather than relying solely on update-date filters. Verified source examples contain corrections that such filters do not identify.
- Reuse the project's HTTP timeout and bounded-retry conventions. A request that still fails is a run failure, rather than a declaration parsing rejection.
- Match the source's numeric Parliament member ID to the existing member's source ID, then store the local member foreign key. Do not match members by name.

### Data model

Use two domain tables, introduced through the existing SQL migration mechanism.

| Table | Field | Meaning |
| --- | --- | --- |
| declarations | id | Stable local UUIDv7 primary key. |
| declarations | source_declaration_id | Unique API declaration/interest ID used for refreshes and grouping. |
| declarations | member_id | Foreign key to the existing member's local UUID. |
| declarations | category_id | Source category identity. |
| declarations | category_name | Human-readable source category name. |
| declarations | fetched_at | Time the accepted source response was retrieved. |
| funding_entries | id | Local UUIDv7 primary key for the current funding row. |
| funding_entries | source_declaration_id | Foreign key to the declaration's unique source ID; shared by all its funding entries. |
| funding_entries | funder | Source-derived donor or payer name, without canonical identity matching. |
| funding_entries | amount | Exact decimal value stored with a decimal-capable SQL numeric type. |
| funding_entries | currency | Source currency code where supplied. |
| funding_entries | payment_type | Source payment-type value where supplied, including monetary or in-kind distinctions. |

- Persist only the fields listed above. Raw responses, unused fields, historical source versions, and alternative payer names are not stored.
- Source responses are transient input for parsing, duplicate comparison, and required parent resolution. Parsing failures are reported through logs only.
- There is one current declaration record per source ID. A later accepted response updates that record; there is no archive of responses or snapshot model.
- There are zero or more funding entries per declaration. A nonfinancial disclosure has a declaration record without fabricated funding entries.
- Preserve legitimately absent optional source values as absent. Missing amounts do not become zero, and missing currency or funder data is not guessed.
- Funding-row UUIDs identify the current local rows. The API does not supply a documented stable identity for nested donor groups.
- A source child payment has its own declaration ID. Use its parent for attribution when required, but store the payment under its own source ID.

### Funding interpretation

- Extract funding from the applicable latest version of the declaration. Do not extract the same funding again from every historical version in the response.
- The selection rule proposed during the design discussion is the newest associated register publication date among the returned versions. Do not assume array order, a maximum register ID, or the declaration's own publication date identifies the latest version. If the choice is genuinely ambiguous and would yield conflicting declaration content, reject and log that declaration.
- This version-selection rule is an application policy. The inspected API contract does not promise version-array ordering; validate the rule against representative recorded responses.
- Preserve the pairing between a funder and its amount within each nested group. Several donor groups produce several funding rows.
- Use an explicitly supplied ultimate payer when present. Otherwise use the appropriate source donor or payer, following the parent declaration when necessary. Store only the selected funder name for each funding entry.
- Resolve required parent information before accepting a child declaration's funding. Do not fabricate a payer or quietly use incomplete information when the required source relationship cannot be resolved.
- Distinguish monetary amounts from unrelated numeric fields. For example, a decimal hours-worked field is not a funding amount.
- Preserve monetary versus in-kind meaning through payment_type. Do not infer cash payment solely because a value has a currency.
- Parse the source's supplied numeric values into exact decimals. Unsupported numeric forms are logged and rejected for now; do not guess, round, or invent a value to complete a declaration.
- A declaration with no funding is valid. A declaration whose funding cannot be completely interpreted is a parsing failure, not evidence that it has no funding.

### Pydantic and module interfaces

- Extend the current separation of source access, model construction/validation, import orchestration, and explicit SQL persistence with declaration-specific behavior.
- Put typed declaration construction and the invariants of the complete funding group in the declaration models, following the existing model-owned member and service rules.
- Decode the page envelope so individual source items remain accessible before validating each declaration. A page model that validates every nested declaration at once would prevent valid siblings from surviving a single bad item.
- Keep source responses in memory while processing the refresh so duplicate checks compare the original content before parsing. Use the existing strict, frozen models for parsed values; neither raw responses nor model JSON dumps are persisted.
- The importer owns source traversal, required relationship resolution, per-run duplicate detection, error handling, and the transaction. Persistence receives a fully accepted declaration and funding group.
- Keep the importer interface comparable to the existing member importer: callers provide database configuration and a source client, and receive a result while progress and failures are logged. Avoid introducing a general ingestion framework or unrelated member refactors.

### Refresh and identity

- Upsert declarations by their source ID and retain the local declaration UUID across refreshes.
- Reparse source responses on each refresh, including responses whose JSON has not changed. Parser improvements must be able to change the normalized funding representation.
- Compare extracted funding by its values and multiplicity. An order-only change does not require replacing otherwise identical rows. Two equal source entries remain two entries; they are not declaration-level duplicates.
- If the funding group is unchanged, retain its rows and UUIDs. Accepted category changes and retrieval metadata can update independently of whether funding changed.
- If funding details change, replace all funding rows for that declaration as one operation inside the enclosing transaction. Removing a source funding entry must remove its old extracted row as part of the replacement.
- Persist the accepted declaration metadata, retrieval time, and resulting funding group together.
- A declaration absent from a response remains untouched. Do not infer a deletion, withdrawal, or a new missing-record status.

### Failure handling and publication

| Condition | Required behavior |
| --- | --- |
| Complete declaration parses successfully | Stage its parsed metadata, retrieval time, and funding together. |
| One funding entry in a declaration cannot be parsed | Reject the entire declaration and log the failure. Do not keep only the valid entries. |
| Existing declaration is rejected | Retain its previous metadata, retrieval time, and funding unchanged. |
| New declaration is rejected | Insert neither a declaration record nor funding rows. |
| Other declarations remain valid | Continue processing them, including siblings from the same source page. |
| Run completes with declaration parsing rejections | Commit accepted declarations and exit successfully; report the rejections through logs. |
| Identical source declaration ID and content repeat within the run | Report and collapse the duplicate before funding expansion. |
| Same source declaration ID repeats with conflicting content | Fail the run and roll back accepted writes from that refresh. |
| Source request fails after retries or its response cannot be read as a page | Fail the run and roll back accepted writes from that refresh. |
| Database write fails or the run is interrupted | Roll back the refresh and use the existing fatal-failure/interruption conventions. |

- Use one database transaction to publish all accepted declaration changes together. Per-declaration parsing rejection is deliberately handled before persistence; it does not abort otherwise valid declarations.
- Member refreshes remain independent. A declaration refresh failure does not undo an earlier successful member refresh.
- Log the rejected declaration's source ID where available, the relevant field path, the original offending value, and the reason. Include enough request/member context to locate an item whose identity itself could not be parsed.
- Declaration diagnostics need the offending numeric input, unlike the existing member error formatter that omits raw values. Keep this diagnostic behavior specific to declaration ingestion.
- Use the established stderr logging and normal command-result conventions. Do not add a partial-success exit code, require a special partial-run workflow, or turn isolated parsing rejections into a failed completed command.
- Execution is controlled externally. Do not introduce advisory locks, application mutexes, or scheduling infrastructure.

## Testing Decisions

### Primary test seam

The agreed primary seam is the declaration importer entry point, analogous to the existing member importer entry point. Exercise source retrieval, model validation, persistence, and transaction behavior through that interface using synthetic Parliament HTTP responses and a real temporary PostgreSQL database initialized by the real migrations.

This is a narrow extension of the existing testing approach rather than a new collection of mockable interfaces. Tests should assert published database state, identity, source requests where they are part of the external API contract, and observable logging. Avoid assertions about private helpers, SQL statement order, internal method calls, or the way models happen to be composed.

Use a small number of CLI checks for command dispatch, stdout/stderr behavior, and exit status. Keep focused HTTP-client tests only for transport behavior such as retry bounds that is clearer at that existing interface. Add direct model tests only where they explain a distinct public validation contract without duplicating importer coverage.

### Prior art

- The member importer already runs against disposable PostgreSQL databases created and migrated for each integration test.
- Synthetic Parliament fixtures use the real HTTP client with a mock transport, and can vary source data between runs or inject request failures.
- Existing tests compare stored data before and after a repeated import, inspect UUID identity, and verify that later source or database failures roll back earlier writes.
- A separate database connection already observes the previously committed dataset while a refresh is in progress. Reuse that approach to prove the declaration publication boundary.
- Current pagination tests tolerate changed totals, and existing client tests verify rate-limit retries and terminal request failures without live network access.

### Required behavioral coverage

1. The command selects the stored cohort once per member, includes former MPs, links declarations to existing members, and leaves member and service data unchanged.
2. Requests explicitly select Commons, include expired records, retain older declarations, and do not restrict ingestion to only one category or the latest register.
3. Readable categories, source IDs, member links, and retrieval times survive persistence. Declaration rows contain only the specified fields, with no raw JSON column.
4. A nonfinancial declaration is stored without funding rows; an unsupported or malformed financial declaration is not silently treated as nonfinancial.
5. Direct donations, in-kind benefits, and nested multi-funder declarations preserve amounts, currencies, payment type, and funder/amount pairings.
6. Ultimate-payer preference and parent-payer fallback produce the expected funder, including when the child is encountered before its parent.
7. Unrelated decimal fields are not funding, and absent optional values do not become invented names, currencies, or zero amounts.
8. Version selection remains correct when the returned array is reordered or versions share their own publication date but reference different register dates. An unresolved conflicting latest choice rejects only that declaration.
9. An unchanged rerun preserves declaration and funding UUIDs. Retrieval metadata may advance without being mistaken for a funding change.
10. Funding edits replace the entire changed group and remove obsolete rows while preserving the declaration UUID. Equal entries retain their multiplicity, and order-only changes do not churn UUIDs.
11. A category-only change updates declaration metadata without replacing unchanged funding. Unused source fields are not persisted.
12. A failure in the second funding entry rejects the whole declaration. Existing metadata, retrieval time, and funding remain unchanged; a rejected new declaration produces no rows.
13. Valid declarations before and after a malformed sibling in the same page still commit. Logs identify the failed source item, field, original value, and reason.
14. A completed run with parsing rejections exits successfully and uses ordinary logs rather than a special partial-success result.
15. Identical duplicate source declarations are reported and collapsed; conflicting duplicates after earlier writes roll back the entire refresh.
16. A later request failure, database failure, or interruption preserves the previously committed declaration dataset. Other connections do not see accepted writes before commit.
17. Changed pagination totals do not fail an otherwise valid refresh, and traversal cannot loop indefinitely on an empty page. There is no inferred deletion or missing-record status for a stored declaration omitted by the source.
18. A stored funding projection that differs from what the current parser derives is corrected on refresh even if the source payload is unchanged.
19. New schema constraints enforce member and declaration relationships and source declaration uniqueness without modifying the member schema or its stored data.
20. Existing member-import tests remain green, demonstrating that the new command has not taken ownership of member refresh behavior.

Automated tests must use deterministic source fixtures rather than depend on live Parliament data. Run the project's existing lint, formatting, type-checking, migration, and test checks when implementing the feature. No new tests are needed merely to verify this specification document.

## Out of Scope

- A display application, serving API, dashboards, donation totals, or financial analysis.
- Canonical donor records, donor-name matching, identity merges, or enrichment from other sources.
- Currency conversion, invented amounts, estimated missing values, or permissive guessing of unsupported numeric formats.
- Refreshing members, changing member import semantics, historical party/constituency attribution, term rollover, or extending the member cohort beyond its existing scope.
- Raw source JSON storage, source snapshots, an archive of previous ingestion runs, event sourcing, and a rejected-record quarantine workflow.
- Deletion/withdrawal inference, missing-record reconciliation, or completeness enforcement based on fixed counts or changing pagination totals.
- Application locking, concurrent-run coordination, scheduling, resumable checkpoints, and a generic ingestion framework.
- Treating per-declaration parsing rejections as a nonzero command exit or a special partial-success workflow.
- A guarantee of complete lifetime history or a frozen upstream snapshot spanning multiple live API requests.

## Further Notes

- In this model, a declaration is a source interest identified by Parliament's ID. A funding entry is one extracted source financial item; several entries can belong to one declaration, and a declaration can have none.
- "Latest" means the latest successfully parsed stored declaration. A rejected newer response leaves the previous complete representation intact. Earlier published versions are used only during interpretation and are not stored.
- The original diagram's separate donor entity has been deferred. Funding entries store the selected source funder name, preferring the ultimate payer when supplied.
- The source API provides declaration identity and category labels, but category-specific fields contain the financial details. The [official v2 schema](https://interests-api.parliament.uk/swagger/v2/swagger.json) documents the register filters, field structure, and source relationships.
- Verified examples informed the model: [an in-kind hospitality declaration](https://interests-api.parliament.uk/api/v2/Interests/16903), [a visit with two funders](https://interests-api.parliament.uk/api/v2/Interests/16716), and [an unpaid trusteeship](https://interests-api.parliament.uk/api/v2/Interests/1220). These demonstrate different shapes, not an exhaustive category catalogue.
- [A payment with an explicit ultimate payer](https://interests-api.parliament.uk/api/v2/Interests/13091) and [a payment that uses its parent's payer](https://interests-api.parliament.uk/api/v2/Interests/5900) establish why preserving parent relationships matters.
- A [verified query for declarations published before the current Parliament](https://interests-api.parliament.uk/api/v2/Interests?Type=Commons&MemberId=172&ExcludeExpired=false&ExpandChildInterests=false&PublishedTo=2024-07-03&Take=20) returned source content. Historical availability therefore extends beyond the current term, without establishing complete historical coverage.
- Actual disappearance from the agreed all-register, include-expired query was not established. The earlier proposed missing-record lifecycle is not part of this spec.
- The specification is intended to be implementation-ready. Its agreed testing seam uses the existing importer-level style; it does not require a separate architecture refactor before development can begin.
