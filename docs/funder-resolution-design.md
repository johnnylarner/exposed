# Declaration funder resolution proposal

This is a design for discussion. The CLI, application, and database are unchanged. Implementation waits until after the user has been taught the attribution policy.

## Problem

The [cleaner](declaration-cleaning.md) preserves each donor, payer, and ultimate payer as a source observation. Those observations are not company identities. A payment can refer to several roles while still representing one funding occurrence. Resolution must answer two separate questions: which observation to attribute a payment to, and whether an observation identifies a company.

The user has scoped company matching to already captured Companies House identifiers. This command will not fetch company records or merge observations by name. Raw and cleaned evidence remains immutable.

## Usage

The proposed command follows the existing declaration stage syntax:

```sh
exposed data declarations resolve exposed/config/declarations-dev.yaml --ingestion-key UUID
```

It reads the cleaned pair and publishes attribution and identity results under the same ingestion directory. It needs `data_dir`, an explicit ingestion key, and no database connection.

```rust
// CLI caller. The storage adapter already owns the ingestion key.
let summary = DeclarationResolverService::new(storage)
    .resolve_declarations().await?;

// Reporting caller. Iterate occurrences once, including unattributed outcomes.
for attribution in resolved.attributions() {
    report(input.funding_entry(attribution.funding_entry_id()), attribution.decision());
}

// Identity caller. A donor can resolve even if a different ultimate payer is selected.
let resolution = resolved.observation_resolution(donor_observation_id);
```

These signatures are sketches, not implemented APIs.

## Attribution policy

Recommend one recorded reporting decision per funding occurrence, while retaining every source role. A selected donor or payer is labelled as such. It is not labelled a verified ultimate source.

The source description of `IsUltimatePayerDifferent` explicitly compares the child's ultimate payer with the payer in its linked parent interest. It does not compare donor with ultimate payer. Local retained source evidence confirms these cases:

- Declaration 5900 has no local name and flag false. Its parent 5222 names `Guardian News & Media Ltd` as payer.
- Declaration 13091 has flag true and names `Viking Penguin`. Its parent 11317 names `Viking- Penguin via Blake Friedmann Literary, TV & Film Agency Ltd`. Keep that text intact.

Apply this proposed policy in order:

| Evidence | Attribution |
| --- | --- |
| Explicit ultimate payer with a usable name | Select that observation as `ExplicitUltimatePayer`. |
| Explicit ultimate payer withheld or blank | Record an unavailable ultimate payer. Do not substitute donor or parent. |
| No ultimate observation and flag true | Record an unnamed different ultimate payer. Do not inherit the parent. |
| No ultimate observation and flag false | Select a unique usable payer from the linked parent. Otherwise record the specific parent failure. |
| No ultimate observation and flag absent, donor observation present | Select the donor if its name or company number is usable. Otherwise record an unavailable donor. |
| No ultimate or donor observation and flag absent, payer observation present | Select that payer if usable. Otherwise record an unavailable payer. |
| None of the above | Record no supported attribution. |

For flag false, look up exactly one parent declaration belonging to the same member. Its eligible payer must occur at the declaration root. That observation may be declaration-scoped or already attached to a top-level funding occurrence. Parse the source pointer once to distinguish the root from nested groups. Do not select an arbitrary donor group, follow grandparents, parse `via` text, or propagate the child's top-level names to its nested payments.

Missing parent evidence, unusable parent payers, multiple parent payer candidates, and self-references produce explicit row outcomes. Detect cycles present in the loaded parent links. Do not claim validation of ancestry absent from the cleaned tables. Cross-member candidates never supply attribution. An absent flag is unknown, so it does not enable parent inheritance. This is stricter than the legacy Python fallback.

An explicit ultimate observation accompanied by flag false is a source disagreement to inspect. Retain the explicit observation as the selected source and record the disagreement. Different spellings alone do not prove different legal entities. The recorded issue must state the conflicting fields rather than assert distinct companies.

This policy is a proposal. Its main product choice is a single useful reporting decision with an explicit basis, instead of making every report choose and repeat its own fallback rules.

## Company identity

Resolve each observation independently of payment attribution. A captured eligible company number yields `source-reported:companies-house:<number>`. Equal keys share a reported company identity even when names differ. Different numbers never merge through matching names.

The number belongs to the observation that supplied it. Current cleaned data attaches numbers only to donors. A payer or ultimate payer with the same name remains unresolved unless it has independent identifier evidence in a future design. A parent reference selects the parent's observation; it does not copy metadata into the child.

Proposed first parser:

1. Trim outer ASCII whitespace and uppercase ASCII letters. Preserve the raw value in the cleaned observation.
2. Accept an already eight-character ASCII alphanumeric token containing a digit as a supported reported identifier. This is an acceptance rule, not a complete registry-format validator. The [Companies House URI guide](https://www.gov.uk/government/uploads/system/uploads/attachment_data/file/426891/uniformResourceIdentifiersCustomerGuide.pdf) supports uppercase prefixes but is too old to supply an exhaustive current prefix list.
3. Keep blank values absent and other forms unresolved. Do not pad short numbers, remove internal punctuation or whitespace, truncate overlong numbers, or guess a registry from a name. Preserve `527227`, `NI 016363`, and overlong values for later correction.

[Companies House number-entry guidance](https://find-and-update.company-information.service.gov.uk/company-lookup/search?forward=/auth-code-requests/company/%7BcompanyNumber%7D/confirm) supports adding leading zeros to shorter values. A later explicit correction rule could use that guidance. The first version leaves short source strings unresolved so that it does not silently turn a possible transcription error into another company's identifier.

An explicit `Company` kind or absent kind permits the source-reported company link. An explicit incompatible kind or an unrecognised kind produces a typed unresolved outcome. Do not turn an individual's number into a company merely because another observation uses it. Missing or withheld names can still carry a reported company number. Personal-looking names, aliases, and conjunctions are neither split nor merged.

Company keys describe agreement with a captured identifier. They do not assert a checked registry entry, a current official name, or verified ownership. Name-only resolution remains outside this first version.

## Shape

The domain model owns company eligibility and attribution. The service exposes one operation. The repository contract owns typed reads and complete publication. The adapter owns Parquet fields and paths.

| Module | Responsibility |
| --- | --- |
| `domain/models/declaration_resolution.rs` | Checked input, typed outcomes, pure resolution, private indexes. |
| `domain/repositories/declaration_resolution.rs` | Read the cleaned dataset and publish the complete result. |
| `domain/services/declaration_resolution.rs` | Coordinate one read, resolution, and publication. |
| `outbound/declaration_resolution.rs` | Decode and check Arrow input, encode output, publish the directory. |
| Existing CLI and `DeclarationIngestionStage` | Add `resolve` and construct filesystem dependencies. |

Reuse `FundingEntryId`, `FunderObservationId`, `FunderRole`, and ingestion errors. Add their required parsing/access traits at the existing owner. Keep wire representations private. Do not revive the generic no-op resolved-data methods or route through website name search.

```rust
struct ReportedCompanyNumber(String);
enum CompanyResolution {
    SourceReported(ReportedCompanyNumber),
    Unresolved(CompanyResolutionReason),
}
enum CompanyResolutionReason {
    NoNumber, UnsupportedNumber, IncompatibleKind, UnsupportedKind,
}
enum AttributionDecision {
    Selected(Selection),
    Unattributed(UnattributedReason),
}
enum Selection {
    UltimatePayer(FunderObservationId),
    ParentPayer { parent: DeclarationId, observation: FunderObservationId },
    Donor(FunderObservationId),
    Payer(FunderObservationId),
}
enum UnattributedReason {
    UltimateMissing, UltimateWithheld, DifferentUltimateUnnamed,
    ParentEvidenceUnavailable, ParentPayerUnavailable, ParentPayerAmbiguous, ParentCycle,
    DonorUnavailable, PayerUnavailable, NoEvidence,
}
struct PaymentAttribution {
    funding_entry: FundingEntryId,
    decision: AttributionDecision,
    issues: Vec<AttributionIssue>,
}
struct ResolvedDeclarations {
    input_fingerprint: InputFingerprint,
    observations: BTreeMap<FunderObservationId, CompanyResolution>,
    attributions: Vec<PaymentAttribution>,
}
impl ResolvedDeclarations {
    fn from_cleaned(input: &ResolutionInput) -> Self;
}
trait DeclarationResolutionStorage: Clone + Send + Sync + 'static {
    fn read_cleaned_declarations(&self)
        -> impl Future<Output = Result<ResolutionInput, EntitySearchPipelineError>> + Send;
    fn publish_resolved_declarations(&self, resolved: &ResolvedDeclarations)
        -> impl Future<Output = Result<(), EntitySearchPipelineError>> + Send;
}
impl<S: DeclarationResolutionStorage> DeclarationResolverService<S> {
    fn new(storage: S) -> Self;
    async fn resolve_declarations(&self)
        -> Result<DeclarationResolutionSummary, EntityIngestionError>;
}
```

`ResolutionInput` has private maps by observation ID, occurrence ID, and member/declaration ID. It carries an `InputFingerprint` of both input file digests and parsed root-scope information. The pure result retains that fingerprint for publication. Construction rejects duplicate IDs, dangling references, invalid role/scope combinations, inconsistent register identities, and malformed required fields. Structurally valid uncertainty becomes an outcome. Storage failure or corrupted structure fails the command.

`ResolutionInput` retains `is_ultimate_payer_different: Option<bool>` on each typed funding occurrence. The nullable flag is already in `funding_entries.parquet`. Its schema extends the captured declaration schema, excluding only `source_json`, and `CleanedFundingEntry` flattens `CapturedFundingEntry`. No cleaner change is needed for payment-level true/false/absent flags. The cleaned pair does not expose every declaration's ancestry, which is why this design supports only the direct parent payer rule.

Selected identity is derived through the selected observation's resolution. It is not copied onto payments as another writable truth. Attribution issues are a typed enum, initially source-flag disagreement. The model's constructors prevent a selection of an unrelated observation. These choices apply Model the Domain. Laziness Protocol keeps one service operation and omits a general matching engine.

## Output and repeat behavior

Publish `observation_resolution.parquet`, `payment_attribution.parquet`, and a manifest together under `<data_dir>/<key>/resolved/declarations/`.

- Observation output has exactly one row per source observation. It records a company key or unresolved reason.
- Attribution output has exactly one row per funding occurrence. It records the selected observation and basis, any parent reference, or an unattributed reason. Issues remain explicit.
- The manifest records input file digests, schema and resolver versions, and row counts. The storage operation must carry the loaded input identity through publication without re-reading a different dataset.

Sort both outputs by stable IDs. Stage the entire result in a unique sibling directory, close the files, and publish with no replacement of an existing result. An existing destination is an error, matching the current cleaner's operator contract. A failed or interrupted attempt leaves no partial final bundle. A later retry uses fresh staging. Supporting several policy versions within one ingestion run can be designed separately.

Consumers sum money by joining funding occurrences one-to-one with attribution. They then look up the selected observation's company key. Unknown identity or attribution stays in an explicit unresolved bucket. Joining every role and adding their amounts would double-count the payment. Declaration-only observations can resolve companies but never create payment rows.

## Acceptance cases

| Case | Expected result |
| --- | --- |
| Donor A has number; ultimate payer B is named | Select B, retain unresolved identity for B, resolve A independently. |
| Ultimate payer withheld; donor A known | Unknown ultimate payer; no donor substitution. |
| True flag with absent ultimate name | Unknown different ultimate payer; no parent substitution. |
| False flag, child 5900, parent 5222 | Select the parent's `Guardian News & Media Ltd` observation once. |
| True flag, child 13091, parent 11317 | Select `Viking Penguin`; keep the parent's agency text unchanged. |
| Eligible parent payer attached to its own top-level funding | Parent selection works without selecting nested payer groups. |
| Flag absent and only parent payer available | Remain unattributed; no inferred false flag. |
| Donor and payer both named, no ultimate observation/flag | Select donor with donor basis; retain payer evidence. |
| Donor withheld, payer named, no ultimate observation/flag | Donor unavailable; do not replace the declared donor with another role. |
| Two donor names with equal accepted number | Same reported company key, separate observations and payments. |
| Equal names with different numbers | Separate company keys. |
| Number `527227` and `00527227` | First unsupported; second a reported key. No silent digit repair. |
| Number `NI 016363`, overlong text, or incompatible kind | Typed unresolved reason; retain raw evidence. |
| Valid reported number with withheld name | Company key with original withheld name status. |
| Parent missing, wrong member, ambiguous, or cyclic | Specific unavailable/conflict outcome, no guessed selection. |
| Identical repeated payments | Separate occurrence IDs and separate attribution rows. |
| Declaration-only company observation | Identity result without an invented payment. |
| Corrupt references or I/O failure | Fail without a completed output. |
| Repeated successful invocation | Existing-output error; previous result unchanged. |

These are proposed implementation tests. They have not been run because no implementation exists.

## Synthesis decision

The parent and independent `gpt-6-astra` cross-judge selected candidate 2, the typed attribution design. Candidates 1 and 2 each scored 23 out of 25 against the design rubric. Persisted attribution decisions broke the tie because the CLI must produce an inspectable answer for offline consumers.

Graft candidate 1's root-scoped parent selection and smaller publication contract. Keep candidate 2's explicit-ultimate selection with a source-disagreement issue. Do not turn differing spellings into proof of different entities. Reject candidate 2's extra unresolved-identity keys and repeated basis fields. The observation ID and selection enum already express those facts.

Candidate 3 scored 13. It retains useful role distinctions but adds traversal APIs and repeats existing cleaned links. Its digits-only parser excludes prefixed company numbers, and it defers parent attribution. It completed before the source-description discovery, which removed that uncertainty. No general graph is needed for this operation. Numeric padding was considered after reading official guidance but deferred following review in favour of the first version's conservative identifier policy.

Exhaust the Design Space led to separate mapping, attribution-decision, and relationship sketches before selecting a design. All were compared against the same observation and payment contracts.

## Tradeoffs accepted

- Accept fewer resolved companies in exchange for identifier evidence without name-based guesses or API enrichment.
- Accept a labelled donor/payer fallback in exchange for useful reporting, while keeping ultimate-payer uncertainty visible.
- Accept one-hop parent support in exchange for a rule justified by retained source descriptions. Broader inheritance needs another design.
- Accept refusing existing output in exchange for immutable results and a smaller first implementation.

## Alternatives considered

An identity mapping with selectable role views is smaller on disk and can centralize policy in its view API. It leaves offline consumers without persisted attribution decisions unless they reproduce or call those views. A relationship model can represent arbitrary roles and ancestry, but the cleaned pair already records these links. Its traversal interface does not settle attribution. A legacy preferred-name string hides why a name was chosen and loses other roles.

## Implementation reconciliation

No implementation has started. The parent accepted the cross-judge's root-scope, outcome, and publication corrections in this proposal. Record any user-approved attribution or identifier policy changes here before implementation.

The final artifact review found no remaining blocking contradiction after checking the flag through schema extension and flattened serialization. CLI syntax and source references were inspected. Application tests were not run because this branch changes only this design document.

## Open questions and risks

Does the single reporting decision with explicit basis fit the intended reports? Should an explicit ultimate payer plus contradictory parent flag be selected with an issue, as proposed, or left unattributed? Source-reported IDs can be wrong even when they have a supported shape. Offline matching cannot resolve that uncertainty.

## Next implementation step

After the teaching checkpoint and user discussion, implement the typed input reader and pure attribution/identity decisions against the acceptance cases.
