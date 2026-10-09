# Declaration funder resolution

`exposed data --config CONFIG declarations resolve --ingestion-key UUID` reads the two cleaned declaration tables and writes identity, attribution, and candidate decisions. Rust owns the policy. A separately installed Python worker uses Splink 4.0.17 with DuckDB 1.4.4 to score feature profiles. The command does not contact Companies House, install packages, or require a runtime database connection.

Resolution answers two independent questions. Observation resolution assigns an identity to each source role. Payment attribution chooses the source role used to report one funding occurrence. Resolving a donor does not make that donor the ultimate source of the payment.

## Input and address evidence

The cleaner replays retained source JSON and checks its existing raw projection. Supplemental addresses attach to the already checked source pointers. The raw Parquet projection and `CapturedFundingEntry` remain compatible with older captures.

At the root, `DonorPublicAddress`, `PayerPublicAddress`, and `UltimatePayerAddress` belong to their respective roles. Within a `Donors` group, `PublicAddress` belongs only to that nested donor. Root addresses never propagate into nested funding. Declaration-only names retain their root addresses without inventing a funding occurrence.

Cleaned observations retain `address_raw`, `address_normalized`, `address_source_field`, and `address_match_quality`. Normalization uses NFKC, lowercase, and collapsed whitespace. Absent values and blank strings remain distinct in raw evidence. Withheld, confidential, private, and not-provided placeholders have no usable normalized address.

`address_match_quality` is `unavailable`, `partial`, or `numbered_street`. The statistical rule requires a numeric house or building token and an explicit street designation. A postcode alone, a place name, or a named premise without a street number remains partial. Some valid international and named-premises addresses therefore require review.

The resolver rejects older cleaned schemas with an instruction to clean retained raw capture into a fresh ingestion run. It checks unique IDs, member and declaration identities, source scopes, role ownership, and reciprocal observation links before scoring. Archived raw files are not needed to resolve a valid cleaned pair.

## Source-reported company anchors

A captured company number is accepted when it has eight ASCII alphanumeric characters and contains a digit. The parser trims outer ASCII whitespace and uppercases ASCII letters. It does not pad short values or remove internal punctuation. `SC012345` and `00527227` are accepted. `527227`, `NI 016363`, and overlong values are not repaired.

Only an explicit `Company` kind or absent kind permits an anchor. Other explicit kinds cannot use their company-number field as a company identity. The resulting key is `source-reported:companies-house:<NUMBER>`. Equal accepted numbers seed the same identity. This means agreement with source evidence, without registry verification.

Individuals, charities, unions, parties, trusts, and other organisations remain eligible for statistical candidates and provisional local identities. Missing donor metadata on payer roles is absence, never a disagreement. Explicit person and organisation kinds prevent an accepted link between incompatible components. `Other` and unfamiliar kinds remain unknown.

## Statistical candidates and acceptance

The frozen model is `funder-frozen-v1`. Its exact settings live in `resolution/exposed_resolution/model.json` and accompany every output manifest. The starting prior is 0.0001. Name comparison has exact, edit-distance-at-most-two, and disagreement levels. Their match probabilities are 0.95, 0.04, and 0.01. Their nonmatch probabilities are 0.001, 0.009, and 0.99. Address comparison has exact and disagreement levels, with match probabilities 0.9 and 0.1 and nonmatch probabilities 0.00001 and 0.99999. Null evidence contributes no weight.

These parameters are chosen starting values. They are not fitted to this dataset, and their computed probabilities are uncalibrated. The worker runs Splink inference directly without unsupervised training or term-frequency adjustments.

Candidate rules use exact normalized names, exact public addresses, a shared three-character name prefix, the same first initial and two-character final-name prefix, or a shared name/alias blocking key. Cleaner-extracted explicit aliases now reach candidate generation as well as acceptance. Generic legal suffixes do not enable the last name rule. The shared `trade-union:unite` key is emitted only for observations explicitly typed `Trade Union` whose source name contains the whole token `Unite`; it brings regional and branch spellings into the candidate set even when the shared root is not at the beginning of the name. Every candidate must have at least one profile containing an observation without an accepted company anchor. Postcodes, member identities, declaration identities, and amounts add no match evidence.

Identical name and address profiles share computation. Every original observation remains distinct until Rust accepts a scored link. A repeated profile uses two distinct worker keys so even its identical-feature comparison receives an actual Splink score. Profile grouping alone never assigns identities. Exact usable donor names can support accepted links independently of their actual scores. The configured `candidate_budget` bounds generated profile pairs and expanded observation pairs. Exceeding the budget fails the command instead of truncating candidates.

Rust accepts an initial statistical edge when supported by one of the following positive-evidence rules, subject to the component constraints below:

- Both normalized names agree exactly, both normalized full addresses have `numbered_street` quality and agree after punctuation/case normalization, and the computed probability is at least 0.999.
- Both are donors with the same usable normalized name, preserving the existing permissive donor rule.
- Explicit aliases agree, or compatible organizations have at least two distinctive shared name tokens plus an agreeing numbered-street address, and one source name has a recognized executive title in parentheses. This supports annotations such as `HSBC UK (Ian Stuart, CEO)` against `HSBC UK Bank plc` while retaining the source name and requiring address corroboration. Shared roots alone do not collapse ordinary committees or branches.
- Both are explicitly typed `Trade Union` observations and both contain the distinctive whole-token root `Unite`. Regional and branch descriptors may differ; each source name and payment role remains attached to its original observation.

The connected unanchored candidate group must not reach different company anchors. The resulting complete component must have at most one distinct company number and no explicit person versus organisation conflict. Address formatting punctuation is ignored when comparing otherwise numbered-street addresses, but address content and ordering remain significant. Roles record where evidence came from; they do not alone contradict identity.

Policy `funder-resolution-v2` accepted exact usable `name_normalized` matches when both observations had role `Donor`, even when addresses were missing, partial, or disagreed. It deliberately accepted same-name false positives and did not use cleaner-extracted aliases or organization components for matching. Policy `funder-resolution-v3` adds the explicit alias, executive annotation, and typed Unite-family rules below. Explicit incompatible kinds and whole-component company constraints still apply across both versions.

Before merging, Rust collects company numbers reachable from each connected group of unanchored observations through eligible links. Groups reaching multiple company IDs keep every company attachment in review, while eligible links within the unanchored group may still merge. This abstention is independent of observation ID ordering. Whole-component checks additionally prevent mixing distinct company numbers or explicit person and organisation kinds through unknown intermediaries.

Fuzzy names without positive corroborating evidence and competing anchors remain review decisions. Rust retains each scored edge's actual probability, comparison levels, disposition, and reason. The new acceptance reasons `extracted_name_evidence` and `trade_union_family` distinguish those domain rules from statistical and exact donor-name acceptance.

Identity membership records whether statistical, exact donor-name, extracted-name, or typed Unite-family evidence connected the component. Redundant donor-name edges do not downgrade a component connected entirely by statistical evidence. Observations supplying an accepted company number retain `source_reported_company`. When a resolved identity is loaded, its displayed source spelling is the most frequently reported nonempty name; all source spellings remain aliases.

A usable name without an accepted link still receives a distinct provisional singleton identity. A singleton is not evidence of a match. A missing or withheld name without an eligible company anchor remains unresolved.

## Payment attribution

Each funding occurrence receives exactly one decision. The policy applies in this order:

| Evidence | Decision |
| --- | --- |
| Explicit ultimate role with a usable name | Select the explicit ultimate payer. |
| Explicit ultimate role with blank or withheld name | Record unavailable ultimate evidence and block fallback. |
| No ultimate role and flag true | Record an unnamed different ultimate payer. |
| No ultimate role and flag false | Select a unique usable same-member root payer in the immediate linked parent. |
| No ultimate role, absent flag, and local donor | Select a usable donor or record donor unavailable. |
| No ultimate role or donor, absent flag, and local payer | Select a usable payer or record payer unavailable. |
| No supported role | Record no supported attribution. |

`IsUltimatePayerDifferent` compares the child's ultimate payer with the payer in its linked parent. It does not compare that ultimate payer with the local donor. Parent selection ignores nested groups and other members. It follows one parent for attribution, while detecting cycles in loaded parent links. It does not infer missing ancestry or parse agency text after `via`.

An explicit ultimate role with flag false stays selected when usable, with a typed source-field disagreement issue. The issue does not claim that different name spellings prove different entities. Declaration 5900 can select parent 5222's `Guardian News & Media Ltd`. Declaration 13091 keeps its explicit `Viking Penguin` name and the parent's original agency text separately.

Consumers sum amounts once by joining funding occurrences one-to-one with `payment_attribution.parquet`. They then join the selected observation to `observation_resolution.parquet`. Joining every role directly to amounts would duplicate payments. Declaration-only observations never create payment rows.

## Publication and repeat behavior

The complete result lives under `<data_dir>/<UUID>/resolved/declarations/`:

| File | Rows or contents |
| --- | --- |
| `observation_resolution.parquet` | One identity decision per input observation, including provisional and unresolved outcomes. |
| `payment_attribution.parquet` | One reporting decision per input funding occurrence, with source basis and issues. |
| `pair_decisions.parquet` | Every eligible scored observation pair, with accepted, review, or rejected disposition. |
| `manifest.json` | Input SHA-256 digests, policy and package versions, exact frozen model, runtime versions, statistical threshold, budget, and counts. |

The manifest retains `automatic_threshold` at 0.999 for the statistical rule. This field does not constrain exact donor-name, extracted-name, or typed Unite-family acceptance. `policy_version` distinguishes that scope through `funder-resolution-v3`; the worker model version is `funder-frozen-v2` because its blocking rules now include explicit alias and typed union-family keys.

The reader hashes the same bytes it decodes. Publication writes all files into a unique sibling directory. macOS and Linux then use atomic rename with exclusive destination creation. Existing results, including an empty destination, are never replaced. Reported failures remove staging. A process killed during staging can leave a hidden directory, but a retry uses a fresh one. No publication lock survives a crash. Other operating systems fail publication with an explicit unsupported-platform error.

Local identities use the ingestion key and the lexicographically first observation ID in an accepted component. Reordering identical inputs preserves IDs. Changing component membership can change a local ID. Different ingestion keys produce different local namespaces. Cross-run reconciliation is outside this command.

## Verification

Rust policy tests cover noncompany identities, exact full-address support, addressless and disagreeing-address donor-name links, mixed evidence, fuzzy and other-role name-only review, sparse-address review, conflicting company anchors, person and organisation conflicts, source-number parsing, reordered input, parent attribution, withholding, and malformed scores. Adapter tests exercise old raw replay, root and nested address isolation, complete Parquet publication, no replacement, malformed worker output, and subprocess failure.

`resolution/tests/test_worker.py` uses the actual pinned Splink package. The ignored Rust integration test `declaration_resolution_cli` creates raw evidence, runs the real `clean` and `resolve` commands, inspects resulting Parquet rows, and checks input immutability and output refusal. It requires the installed worker and a build environment that can satisfy existing SQLx macros. See the [runtime setup](../resolution/README.md) for both commands.

Verification runs the complete Rust library suite with an isolated PostgreSQL database, the real pinned Splink worker tests, and the clean-to-resolve CLI integration test. The CLI fixture verifies statistical charity links and Gary Lubner donor links across missing and disagreeing addresses with real probabilities below 0.999. It joins results to source names to check group membership and pair endpoints, and verifies unchanged cleaned observations and funding occurrences and refusal to overwrite results.
