# Declaration funder resolution

`exposed data declarations resolve CONFIG --ingestion-key UUID` reads the two cleaned declaration tables and writes identity, attribution, and candidate decisions. Rust owns the policy. A separately installed Python worker uses Splink 4.0.17 with DuckDB 1.4.4 to score feature profiles. The command does not contact Companies House, install packages, or require a runtime database connection.

Resolution answers two independent questions. Observation resolution assigns an identity to each source role. Payment attribution chooses the source role used to report one funding occurrence. Resolving a donor does not make that donor the ultimate source of the payment.

## Input and address evidence

The cleaner replays retained source JSON and checks its existing raw projection. Supplemental addresses attach to the already checked source pointers. The raw Parquet projection and `CapturedFundingEntry` remain compatible with older captures.

At the root, `DonorPublicAddress`, `PayerPublicAddress`, and `UltimatePayerAddress` belong to their respective roles. Within a `Donors` group, `PublicAddress` belongs only to that nested donor. Root addresses never propagate into nested funding. Declaration-only names retain their root addresses without inventing a funding occurrence.

Cleaned observations retain `address_raw`, `address_normalized`, `address_source_field`, and `address_match_quality`. Normalization uses NFKC, lowercase, and collapsed whitespace. Absent values and blank strings remain distinct in raw evidence. Withheld, confidential, private, and not-provided placeholders have no usable normalized address.

`address_match_quality` is `unavailable`, `partial`, or `numbered_street`. The conservative automatic rule requires a numeric house or building token and an explicit street designation. A postcode alone, a place name, or a named premise without a street number remains partial. Some valid international and named-premises addresses therefore require review.

The resolver rejects older cleaned schemas with an instruction to clean retained raw capture into a fresh ingestion run. It checks unique IDs, member and declaration identities, source scopes, role ownership, and reciprocal observation links before scoring. Archived raw files are not needed to resolve a valid cleaned pair.

## Source-reported company anchors

A captured company number is accepted when it has eight ASCII alphanumeric characters and contains a digit. The parser trims outer ASCII whitespace and uppercases ASCII letters. It does not pad short values or remove internal punctuation. `SC012345` and `00527227` are accepted. `527227`, `NI 016363`, and overlong values are not repaired.

Only an explicit `Company` kind or absent kind permits an anchor. Other explicit kinds cannot use their company-number field as a company identity. The resulting key is `source-reported:companies-house:<NUMBER>`. Equal accepted numbers seed the same identity. This means agreement with source evidence, without registry verification.

Individuals, charities, unions, parties, trusts, and other organisations remain eligible for statistical candidates and provisional local identities. Missing donor metadata on payer roles is absence, never a disagreement. Explicit person and organisation kinds prevent an accepted link between incompatible components. `Other` and unfamiliar kinds remain unknown.

## Statistical candidates and acceptance

The frozen model is `funder-frozen-v1`. Its exact settings live in `resolution/exposed_resolution/model.json` and accompany every output manifest. The starting prior is 0.0001. Name comparison has exact, edit-distance-at-most-two, and disagreement levels. Their match probabilities are 0.95, 0.04, and 0.01. Their nonmatch probabilities are 0.001, 0.009, and 0.99. Address comparison has exact and disagreement levels, with match probabilities 0.9 and 0.1 and nonmatch probabilities 0.00001 and 0.99999. Null evidence contributes no weight.

These parameters are chosen starting values. They are not fitted to this dataset, and their computed probabilities are uncalibrated. The worker runs Splink inference directly without unsupervised training or term-frequency adjustments.

Candidate rules use exact normalized names, exact public addresses, a shared three-character name prefix, or the same first initial and two-character final-name prefix. Generic legal suffixes do not enable the last rule. Every candidate must have at least one profile containing an observation without an accepted company anchor. Name transformations are candidate rules or levels of one name comparison. Postcodes, roles, member identities, declaration identities, and amounts add no independent match evidence.

Identical name and address profiles share computation. Every original observation remains distinct until Rust accepts a scored link. A repeated profile uses two distinct worker keys so even its identical-feature comparison receives an actual Splink score. Grouping alone never merges name-only observations. The configured `candidate_budget` bounds generated profile pairs and expanded observation pairs. Exceeding the budget fails the command instead of truncating candidates.

Rust accepts an initial statistical edge only when all of these conditions hold:

- Both normalized names agree exactly.
- Both normalized full addresses agree exactly and have `numbered_street` quality.
- The computed probability is at least 0.999.
- Neither endpoint has competing strong matches to different company anchors.
- The resulting complete component has at most one distinct company number and no explicit person versus organisation conflict.

Fuzzy names, missing or partial addresses, name-only evidence, and competing anchors remain review decisions. Conflicting full addresses or incompatible kinds produce rejected decisions. Rust retains every eligible scored edge with its probability, comparison levels, disposition, and reason. Deterministic ordering and whole-component checks prevent an unnumbered observation from bridging different company IDs. Reviewed-pair calibration is required before widening automatic matching.

A usable name without an accepted link still receives a distinct provisional singleton identity. A singleton is not evidence of a match. A missing or withheld name without an eligible company anchor remains unresolved. In the inspected source capture, named individuals lack usable public addresses, so this initial policy supplies review candidates and provisional identities for them.

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
| `manifest.json` | Input SHA-256 digests, policy and package versions, exact frozen model, runtime versions, threshold, budget, and counts. |

The reader hashes the same bytes it decodes. Publication writes all files into a unique sibling directory. macOS and Linux then use atomic rename with exclusive destination creation. Existing results, including an empty destination, are never replaced. Reported failures remove staging. A process killed during staging can leave a hidden directory, but a retry uses a fresh one. No publication lock survives a crash. Other operating systems fail publication with an explicit unsupported-platform error.

Local identities use the ingestion key and the lexicographically first observation ID in an accepted component. Reordering identical inputs preserves IDs. Changing component membership can change a local ID. Different ingestion keys produce different local namespaces. Cross-run reconciliation is outside this command.

## Verification

Rust policy tests cover noncompany identities, exact full-address support, fuzzy and name-only review, sparse-address review, conflicting company anchors, person and organisation conflicts, source-number parsing, reordered input, parent attribution, withholding, and malformed scores. Adapter tests exercise old raw replay, root and nested address isolation, complete Parquet publication, no replacement, malformed worker output, and subprocess failure.

`resolution/tests/test_worker.py` uses the actual pinned Splink package. The ignored Rust integration test `declaration_resolution_cli` creates raw evidence, runs the real `clean` and `resolve` commands, inspects resulting Parquet rows, and checks input immutability and output refusal. It requires the installed worker and a build environment that can satisfy existing SQLx macros. See the [runtime setup](../resolution/README.md) for both commands.

The implementation session ran the actual domain, replay, and filesystem modules through an isolated harness that excludes database adapters. Full CLI execution and real Splink inference remain blocked in this environment by unavailable SQLx database access and unavailable Python dependencies. Mock scores prove Rust policy only.
