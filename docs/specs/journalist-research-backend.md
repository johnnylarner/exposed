# Journalist research backend

Status: product scope agreed; implementation defaults proposed below.

Prepared: 17 September 2026.

Repository baseline inspected: `52c20d37a2f88f4e9b12cb591092da497fa9efa7`.

This is a specification for a person implementing the backend by hand. It defines the
behaviour to build, the evidence available today, and the boundaries that should make
the work manageable. The serving backend will be written in **Rust**, using hexagonal
architecture. The web framework, runtime and database driver remain implementation choices.

**Start with [the first implementation slice](#the-first-implementation-slice).**
Read the product scope and the data rules before designing an aggregate query.
The separate handoff is a navigation aid; this document is the durable product reference.

## The product we chose

Exposed will be a research and reference tool for journalists. Success means reporters
return to it for investigations and link to it from publications.

> We help journalists investigate MPs' financial interests and hold political power to account.

The agreed first version has four main surfaces:

| Surface | What the backend needs to support |
| --- | --- |
| Entity search | A single autocomplete showing both MPs and funders. Selecting a suggestion opens that entity's page. |
| MP page | Identity, financial summaries, associated funders, declaration history and source references. |
| Funder page | Total declared value, value by party, the top five MPs, the latest five registered declarations, full declaration history and MPs over time. |
| Rankings page | Dynamic, filterable and sortable lists. Begin with MPs; funder-name and party groupings use the same existing data. |

These decisions are settled:

- MP search and funder search are both first-class capabilities. The interface selects
  entities; it is not a general full-text search through declaration narratives.
- Funders initially mean names recorded in the data. They can be organisations or people.
- The headline measure combines available declared financial values across categories,
  including monetary and in-kind entries. Categories remain available for inspection
  and optional filtering; selecting a category is not required to obtain a total.
- Recency means registration date. The earlier request for “diffs” became a
  registration-date filter, not comparison of old and new versions.
- The latest five declarations introduce the full history.
- “MPs over time” means MPs appearing in a funder's declarations over registration dates.
- Rankings live on a separate page. Rank tags on profiles are outside the MVP.
- Start with existing data. Organisation resolution, alias merging, industries, career
  histories and historical party enrichment come later.

Default scope guardrails: this slice does not add a graph interface, original editorial
publishing workflow, accounts, saved alerts, immutable citation snapshots or ingestion
scheduling. These are exclusions for a focused implementation, not requirements for
the product's eventual lifetime.

Search and evidence are the core journey. The mission page gives the product its public
purpose, but does not require a separate backend capability.

## What exists today

The existing application is a Python 3.14+ ingestion tool using PostgreSQL. Its core,
ports, adapters and composition root already follow hexagonal architecture.
There is no serving backend or HTTP framework in the inspected repository.
The new Rust application will read the existing PostgreSQL data; ingestion remains Python.

Read these as the source of truth for existing behaviour:

| Reference | Why it matters |
| --- | --- |
| [Database baseline](../../db/migrations/20260915000000_initial.up.sql) | Actual tables, nullable fields and identities. |
| [Ingestion README](../../ingest/README.md) | Cohort, dates, refresh behaviour and source coverage. |
| [Ingestion architecture](../architecture.md) | Existing boundaries and tests worth following. |
| [Declaration ingestion spec](declaration-ingestion.md) | Existing writer contract; this new serving feature does not change it. |
| [Recorded data audit](../reviews/2026-09-17-data-consistency.md) | Observed data and limitations, not a current database inventory. |
| [Payment-period notes](../notes/payment-periods.md) | Why stored amounts are not comparable earnings received over a period. |
| [Funder-name notes](../notes/funder-names.md) | Name variants and disclosure placeholders. |

The usable records are:

- **Members:** stable Parliament member ID, name, latest recorded party, House and
  membership location, and current Commons status. Stored service periods identify
  the configured Parliament's cohort, including former MPs.
- **Declarations:** stable source declaration ID, owning MP, category, nullable
  registration date and retrieval timestamp.
- **Funding entries:** declaration association, selected funder name, nullable exact
  amount, currency and payment type. One declaration can have several entries or none.

The database does not retain full declaration wording, payment regularity, payment/event
dates, all alternative payer names, parent relationships, source versions or import-run
history. Source material can be linked to, but cannot be reproduced as a stored archive.

The recorded audit found all funding currencies were GBP and some registration dates
were missing. Treat those as observations. The nullable schema and future imports remain
the contract.

## Rules for a trustworthy query

The scope above was agreed in conversation. **Defaults** in this section fill in details
needed to begin implementation; they are recommendations, not additional user commitments.

### Identity and funder search

**Default:** use Parliament's numeric member ID for an externally stable MP reference,
and the source declaration ID for a declaration reference. Existing local UUIDs remain
storage identities.

Give every search result an explicit kind, reference and display name. MP and funder
references must not be interchangeable.

A funder reference identifies an **exact stored name**, not a verified company. Its
representation should be deterministic and round-trip safely, including Unicode,
punctuation and spaces. An opaque URL-safe encoding of the exact name is sufficient
initially. It must not use a funding-entry UUID: those rows can be replaced on refresh.

Matching can be case-insensitive without changing identity. Searching a name can find
several case or spacing variants; display them separately and preserve their original
labels. Do not merge subsidiaries, aliases or people who happen to share a name.

**Default search behaviour:**

- Return separate MP and funder suggestion groups, with at most five of each initially.
- Trim the query; fewer than two characters returns empty groups. Reject queries over
  200 characters. These limits are adjustable usability defaults.
- Match literal name text case-insensitively: exact matches, then prefixes, then
  substrings. Break ties by display name and stable reference.
- Treat wildcard characters as literal user text, not query syntax.
- Search the stored cohort independently of a profile's date filter. An entity can
  exist even when the selected registration period has no matching declarations.
- MP context can include recorded party and membership information. Only describe
  `latest_membership_from` as a constituency when the latest House is Commons.
- Funder suggestion counts, if supplied, describe stored records and distinct MPs.
  They are optional for the first slice.

Null/blank funders and recognised disclosure placeholders must not become fictional
entities. The notes document `Confidential` and case variants: a small explicit,
reviewable placeholder policy can handle these without attempting entity enrichment.
Preserve their original text in declaration evidence. Their amounts still contribute
to MP and party totals, and can be reported as unattributed funding.

A source-name page can disappear if its last stored name is corrected on refresh.
Its reference must never silently resolve to a different name. Alias redirects and
historical name preservation are later work.

### Dates, cohort and filters

Use a shared validated scope for summaries, histories and rankings:

- Registration start/end dates, independently optional.
- Optional recorded party and declaration category filters.
- The selected currency for the primary numeric measure and amount sorting.
- An entity subject where relevant: one MP or one exact funder name.

Currency selects the monetary measure, not the evidence set: declarations in other
currencies remain in histories and declaration counts, with their own subtotals.

**Defaults:** date bounds are inclusive and use calendar dates; no bounds means all
available stored records. Reject a start after the end. Show the exact bounds in results.
“Registered on or after” is clearer than an ambiguous “after”.

When a date bound is supplied, undated declarations do not match it. Return their
excluded count under the other filters so the omission is visible. In an unbounded
history, include them after dated records. Do not substitute retrieval, publication or
payment dates.

Keep registration chronology distinct from an archive of changes. A query reads the
latest stored representation of declarations with those registration dates.

**Default cohort:** all stored members who served in the configured Parliament, including
former MPs. A member with several service periods counts once. A declaration date filter
does not imply that its owner held office throughout that date range.

Coverage describes that cohort and its available declarations. It is not a complete
all-time archive of every MP. Party attribution uses the MP's latest recorded party,
not their party when a payment or registration occurred. Include an unknown-party
bucket so group totals remain explainable.

### What “total declared value” means in this MVP

The product wants a combined amount across declaration categories. Preserve that choice.

**Proposed calculation:** sum the stored, known funding-entry amounts in the selected
scope and currency, including monetary and in-kind values. Describe the measure as a
**sum of stored declared values**, with the user-facing label “Total declared value”.

There is a material interpretation limit. The [payment-period investigation](../notes/payment-periods.md)
found monthly, quarterly and yearly source amounts, while regularity is not retained.
One source description also qualified its period label. Therefore this sum:

- does not measure money received during the registration window;
- is not an annualized or period-normalized earnings figure;
- cannot establish comparable economic benefit merely because entries share a currency.

The backend should make the measure definition available with aggregate results.
Do not quietly annualize, estimate missing amounts or rename the sum “total income”.
A source-value ranking can be implemented now. Claims about comparable payments or
benefits received require further source interpretation before publication.

Calculation rules:

1. Use exact decimal arithmetic. Represent decimal values exactly at the transport
   boundary, for example as decimal strings; do not accumulate binary floating-point values.
2. Keep currency totals separate. Default the primary measure to GBP because it matches
   the recorded dataset, but report other known currency subtotals. No conversion is required.
3. Keep unknown amounts/currencies explicit. Return missing-amount counts by known
   currency and an unknown-currency entry count. A genuine source zero remains zero.
4. Count each stored funding entry once. Preserve legitimate equal entries; never
   deduplicate by amount or by the tuple of displayed fields.
5. Prevent joins to service periods or funding rows from multiplying values or declaration
   counts. A declaration count means distinct source declaration IDs, not funding rows.
6. An MP total includes their matching entries. A **funder total includes only entries
   attributed to that exact name**, even when the parent declaration has other funders.
7. Current storage cannot establish whether separate source IDs describe overlapping
   economic payments. Do not invent cross-declaration deduplication. This is another
   limit of the source-value measure.
8. Aggregate the full matching set before applying the five-item preview or page limit.

No matching records, records with unknown amounts, and a known zero amount are distinct
outcomes. If a group has matching entries but no known amount in the selected currency,
its value is unavailable and sorts after known values. A member with no matching
declarations remains visible in an all-MP list with that status; do not present missing
records as proof of no financial interests.

**Default result convention:** a selected-currency total is null when it has no known
contributing amounts. Return the contributing-entry count and matching-declaration
count beside it. A numeric zero requires known contributing amounts that sum to zero.

### Drill-down, history and sources

Every displayed aggregate must be traceable to the declarations and funding entries
that contributed to it. A funder-scoped declaration response should expose both its
matched contribution and the full recorded funding context, with the distinction explicit.

The same scope governs headline totals, party breakdowns, top MPs, recent declarations
and the temporal view. Top-five limits affect display, not the totals.

Default declaration ordering is registration date descending, then source ID descending;
null dates appear last when included. Rank by selected-currency declared value descending,
then display name and stable reference. Equal amounts remain visibly equal.

**Default sort choices:** histories support registration date ascending/descending;
rankings support declared value, name and declaration count in either direction.
Keep unavailable values last in either direction, and apply stable reference tie-breaks.
Top-five summaries always use value descending and omit groups with no available value;
their absence must remain visible in the supporting counts and full lists.

**Default pagination:** offset/limit with a default of 50 and a maximum of 100 is enough
to start. Validate both. Use the same ordering for preview and full history. Stable
ordering is guaranteed for an unchanged dataset; a later request can observe refreshed data.

“MPs over time” returns dated registration events with MP and declaration references,
plus the matching funder values. One declaration is one event even if it has several
matching funding entries. Default event order is date ascending, then source ID.
Expose undated counts separately. First/last dates mean first/last available registrations
in the selected scope, not the beginning or end of a real-world relationship.

Include source declaration IDs, registration dates and record retrieval timestamps in
evidence responses. Source URLs should be based on verified Parliament link formats;
the [source research](../research/uk-parliament-apis.md) and payment-period notes contain
official API examples. Label an API JSON link as such. Do not fabricate a human-readable
per-declaration web URL or imply the full text is stored here.

Entity references and explicit filters should support linkable pages. These are live
views, not immutable publication snapshots. A retrieval timestamp describes that record,
not a successful complete import of the whole dataset.

## Application capabilities

These are inbound use cases, not prescribed HTTP endpoints. Their Rust inputs and
results belong to the research core.

| Capability | Inputs | Meaningful result |
| --- | --- | --- |
| Search entities | Query and suggestion limits | Separate typed MP and funder matches; deterministic order; no match is a normal empty result. |
| Read MP page | MP reference, scope | Recorded identity, totals, category context, associated funders and first history page. |
| Read funder page | Exact-name reference, scope | Total values, declaration/MP counts, party breakdown, top five MPs and five recent declarations. |
| List declarations | MP/funder subject, scope, page request | Complete matching history, source references and clear matched funding contributions. |
| Read declaration | Source declaration ID | Current stored record, owning MP and recorded funding, for evidence links. |
| List rankings | MP, funder-name or party grouping; scope; sort; page | Dynamic ordered groups and their values/counts. Implement MPs first. |
| Read funder's MPs over time | Exact-name reference, scope, page request | Registration events suitable for grouping by MP, with evidence references and pagination metadata. |

A page response should carry its effective filters, measure/currency, counts and relevant
data limitations. Frontends should not have to reconstruct business definitions from
the raw rows.

Expected outcomes are explicit:

- Invalid query, date range, identifier shape, grouping, sorting or pagination:
  an application validation error.
- A well-formed reference that does not exist: entity/declaration not found.
- An existing entity with no declarations in a valid scope: successful empty result.
- A storage failure or invalid stored record: a dependency/data failure, not zero totals
  or an empty search result.

An HTTP adapter can map these to 400, 404, 200 and an appropriate 5xx response respectively.
Keep driver messages and configuration details out of public responses; retain diagnostic
causes for the operator.

## Hexagonal shape

Add a cohesive **research/read application** alongside ingestion. Keep ingestion as
the writer of its existing data and preserve its commands, transactions and validation.
This work does not require a migration of the already-hexagonal importer.

| Boundary | Responsibility |
| --- | --- |
| Domain values | Typed references, registration windows, decimal amounts, currencies, valid scopes and expected failures. |
| Research use cases | Search, entity pages, histories and rankings; interpretation and ordering policies; application results. |
| Core-owned read port | Obtain the projections these use cases need, under their filtering, counting and consistency contracts. |
| PostgreSQL adapter | SQL, efficient grouping, row-to-core conversion, read transactions and dependency-error translation. |
| Inbound adapter | Parse transport inputs, call use cases, serialize results and map failures. |
| Composition root | Load configuration, create connections/adapters, inject the read port and own lifetimes. |

One narrow `ResearchReadPort` is a reasonable starting point. Give it operations
describing required projections, such as entity matches or a funder-page projection,
rather than generic CRUD repositories for every table.

The core owns **what** a total or match means. SQL may execute the aggregate efficiently
under that contract; hexagonal architecture does not require loading the entire dataset
into application memory. Adapter contract tests must prove the filtering and counting
rules rather than leaving those rules implicit in SQL.

Use validated core result types on the port. Do not expose ORM queries, cursors,
connection/transaction objects, web requests or database exceptions to the core.
Rehydrate stored data through the same applicable invariants. Carry the existing domain
conventions into Rust values; the serving application does not depend on Python modules.

A composed page must be internally consistent: metadata, totals, previews and rankings
within that response must observe one committed database snapshot. The read adapter can
implement this with a single query or a suitable read transaction. Independent requests
can observe later committed data. Because ingestion commits declarations per MP, this
does not promise an atomic refresh of the whole register.

Requests read stored data. They do not trigger a Parliament fetch, ingestion, database
migration or schema reset. No queues, outboxes, write ports, precomputed rank tables,
generic unit-of-work framework or search service are required to demonstrate this slice.

### Rust implementation starting point

**Recommended layout:** one Cargo package under `backend/`, with a library target for
testable modules and a small binary entry point. Keep domain values, use cases and ports
under `core`; transport and PostgreSQL code under `adapters`; construction under
`composition`. Let `main.rs` handle startup and lifecycle through that composition root.
Expose the library surface integration tests need, including adapter construction.
Separate crates can wait until a concrete boundary needs compiler-enforced isolation.

Start with a use-case service generic over the core-owned read trait. Production injects
the PostgreSQL adapter; core tests inject a fake. Use private fields and fallible
constructors or `TryFrom` for validated references, scopes and page requests. Decode
transport and database representations at their adapters, then construct core values.
Deserialization must not provide a route around validation.

Represent expected failures with typed `Result` errors. Preserve a storage failure's
diagnostic cause inside the adapter boundary without exposing driver-specific types
in the core contract or driver messages in public responses. Use optional values for
missing amounts/dates, exact decimal values for money and enums for entity kind and sort
choices. Operational failures should not become panics.

Decide whether port operations are asynchronous when choosing the runtime and driver.
Async operations and dynamic dispatch are separate choices. For the proposed generic
service, native async trait methods or returned futures can suffice. If the runtime
requires sendable futures, express `Send` on the returned future: a bare async trait
method does not promise that bound to generic callers. See the
[Rust async-trait guidance](https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits/).
If dynamic dispatch later becomes necessary, use a compatible erased-future signature;
native async methods cannot be called through `dyn` under the documented
[trait compatibility rules](https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility).

The framework, runtime and driver are still open. Keep their types at the edges, choose
only dependencies needed for the first slice, and confirm their current APIs against the
chosen Rust toolchain. No Cargo package or toolchain pin exists yet. Compile the actual
handler-to-service-to-adapter composition early so ownership and future bounds are
checked where the application will run.

## The first implementation slice

**Tomorrow's first finish line: type a name, receive both kinds of suggestion, and
select either kind to retrieve its identity and recorded declarations.**

This deliberately starts with the product's main entrance. The aggregate interpretation
work can then be tackled with a functioning, testable read path.

1. Sketch the Rust module boundaries above and create the small serving package. Select
   its toolchain, entry mechanism and adapter dependencies. Read the hexagonal skill's
   bootstrap workflow and Rust reference, preserving the existing ingestion application.
2. Define only the reference, search input/result and failure types needed for this path.
   Give the read port a bounded search operation and typed selection/history reads.
3. Exercise the use case with a small fake: both result kinds, literal matching,
   no match, invalid input, stable selection and a meaningful storage failure.
4. Implement the PostgreSQL adapter against the existing schema. Test it with a
   disposable database so joining, names and identifiers are proved against real SQL.
5. Add a thin inbound adapter and production composition. Demonstrate the full path
   for one MP and one funder through the actual entry mechanism.

Done means both selections reach real stored declarations, the core runs with a fake,
real adapter tests pass, errors remain distinguishable, and no ingestion behaviour has
changed. A stub returning fabricated search results does not meet that finish line.

Continue in these useful increments:

| Increment | Visible outcome |
| --- | --- |
| Shared scope and complete histories | Registration/category/party filtering, null handling, stable pagination and evidence links. |
| Aggregation contract | Exact selected-currency source-value sums and counts, proved using the example below. |
| Funder and MP summaries | Party breakdowns, top-five MPs, linked funders and recent records all reconcile with their evidence. |
| Dynamic rankings | Start with all MPs, then the existing-data funder-name and party views, using the same measure and scope. |
| MPs over time | Registration events provide the agreed funder timeline without a new enrichment model. |

## An arithmetic example to implement against

All names and records below are synthetic. MP A has recorded party P; MP B has party Q.
Each funding item belongs to the declaration on its row.

| Declaration | Registered | MP | Recorded funding |
| --- | --- | --- | --- |
| D1 | 2026-09-01 | A | Funder A: GBP 100 monetary; Funder B: GBP 30 in kind |
| D2 | 2026-09-02 | A | Funder A: GBP 20 in kind |
| D3 | 2026-09-03 | B | Funder A: GBP 50 monetary |
| D4 | Unknown | A | Funder A: GBP 10 monetary |
| D5 | 2026-09-04 | B | Funder A: EUR 7 monetary |
| D6 | 2026-09-05 | A | Funder A: unknown amount, GBP |
| D7 | 2026-09-06 | A | Nonfinancial declaration, no funding entries |

For Funder A, registered from 1 through 30 September 2026, primary currency GBP:

- Headline known GBP source-value sum: **170**, comprising 100 + 20 + 50.
- Separate EUR subtotal: **7**. One matching GBP funding amount is unknown.
- Matching declarations: **5**; distinct MPs: **2**. D4 is excluded because its date
  is unknown; report that one excluded declaration.
- Party P's GBP value: **120**; party Q's: **50**. Their declaration counts are
  3 and 2 respectively, including the matching non-GBP and unknown-amount records.
- Top MPs by known GBP sum: A (**120**), then B (**50**).
- Recent history: D6, D5, D3, D2, D1. Currency chooses the measure, not which evidence
  records exist; D5 remains visible with its EUR amount.
- D1 contributes **100**, not 130, to Funder A's total. The evidence still exposes
  the other funding item as context.
- MP A's own dated page contains D1, D2, D6 and D7, with known GBP sum **150**.
  The nonfinancial declaration remains in that history.
- Removing the date bounds adds D4: Funder A's known GBP sum becomes **180**.

This example specifies source-value arithmetic, not an estimate of actual income.

## Behaviour to verify as implementation grows

| Boundary / case | What should be observable |
| --- | --- |
| Core values | Invalid dates, references, page bounds and sort options are rejected independently of HTTP/SQL. |
| Search | Both entity kinds appear; exact/prefix/substring order is stable; punctuation is literal; name variants remain separate. |
| Identity | Selected references round-trip; a funding-row replacement does not change a surviving funder-name reference. |
| Placeholders | Confidential/unknown entries stay in source evidence and MP values without becoming named funder entities. |
| Aggregation | The synthetic example reconciles; distinct declarations are counted once and legitimate repeated funding items retain multiplicity. |
| Join safety | Additional service periods and multi-funder declarations do not inflate counts or values. |
| Dates and attribution | Inclusive bounds, missing dates and latest-recorded party attribution behave as documented. |
| History | Preview and full history share scope/order; pages are stable for unchanged data; a nonfinancial declaration remains visible. |
| Failures | Unknown entity, valid empty scope and storage failure produce different outcomes. |
| Snapshot | A composed page does not mix a pre-refresh total with post-refresh evidence within one response. |
| Inbound adapter | Parsing and public error/decimal/date serialization work against a substituted use case. |
| Composition | One small real-database smoke check exercises the production wiring for both search targets. |
| Dependency direction | The core imports no HTTP framework, database driver, configuration loader or concrete adapter. |

Use focused domain/use-case tests, meaningful real PostgreSQL adapter tests and a small
wiring check. An in-memory fake does not prove SQL correctness or snapshot behaviour.

## Working in this repository

Follow [AGENTS.md](../../AGENTS.md): use the dedicated worktree, keep history linear,
and commit completed, validated changes on the work branch. Preserve the primary checkout.

The current [setup commands](../../README.md#getting-started) install and configure the
ingestion application. The [database guide](../../db/README.md) describes its development
baseline. Reading the existing data does not require resetting it or running a new import.
If schema work becomes necessary, make that a separate deliberate change.

Existing commands, from the repository root:

- `make test-unit`: ingestion tests without PostgreSQL.
- `make check`: ingestion lint, formatting, typing and tests; requires the Python
  environment and local PostgreSQL.
- `git diff --check`: whitespace validation for changed files.

Those commands do not yet validate the planned Rust backend. Once its package exists,
run the following from that package directory:

```sh
cargo fmt --all -- --check
cargo check --all-targets
cargo test
cargo clippy --all-targets -- -D warnings
```

These are implementation checks to wire up tomorrow; no Rust application has been
created or tested as part of this spec. Document the chosen application's run command
and disposable PostgreSQL setup alongside it. For existing database-test patterns, read
[the test fixtures](../../ingest/tests/conftest.py); the Rust adapter tests will need
their own fixture setup. For dependency-check ideas, read
[the architecture tests](../../ingest/tests/test_architecture.py).

The first slice can begin without enrichment. Before presenting stronger financial
comparisons publicly, revisit payment-period semantics, possible cross-record overlap,
coverage and original-source link usability. Preserve those limitations in the response
contract instead of making up facts the current storage cannot establish.
