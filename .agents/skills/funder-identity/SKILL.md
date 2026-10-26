---
name: funder-identity
description: Check Exposed company identity conflicts, type and identifier precedence, and parent-payer selection with bounded Kani proofs and exhaustive Rust checks. Use after changes to source funding fields or declaration resolution. Report the two kinds of evidence separately.
---

# Check funder identity rules

From the repository root, run:

```sh
python3 .agents/skills/funder-identity/scripts/check.py
```

The command runs nine exhaustive Rust checks and eleven Kani harnesses against
the current production source. A successful command requires both suites to pass.
Run `--mutations` when changing this skill or its assertions. It changes disposable
copies of the real source, requires the predicted assertion failures, and reruns
the original suites. It never edits the checkout's production files.

Install the pinned verifier if it is absent:

```sh
cargo install --locked kani-verifier --version 0.68.0
cargo kani setup
```

Use Python 3.10 or newer and Cargo. Kani's setup supplies its Rust toolchain and
CBMC. The validated platform is Apple Silicon macOS. Other platforms supported
by Kani need their own successful run. The proof crate has a separate workspace
and lockfile. It needs no database, application server, or ingestion artifacts.

## Properties and scope

Conflicting captured company identifiers must not become one resolved identity.
A violation can attribute declarations to the wrong organisation. Explicit person
and organisation evidence must not merge through an unknown intermediary.
Company identifiers take precedence over name-based fallback identifiers.

The native suite enumerates all 41,055 graphs with one to four observations.
Each observation has one of five fixtures: unknown type, company `00000001`,
company `00000002`, person, or organisation. Every subset of undirected edges is
included. All observations are donors with the same normalized name and no
address. This exercises actual edge expansion, classification, component merging,
and output selection. It is exhaustive for this fixture domain, not a Kani proof
or a claim about arbitrary strings, graph sizes, or scoring policies.

A separate 64-graph check requires ambiguous unanchored bridges between two
companies to abstain. Parent-selection checks enumerate all 48 combinations of
member, immediate parent, root scope, payer role and the three flag states.
They also cover six explicit-ultimate cases, four missing/withheld donor cases,
and eight parent-cycle/ambiguity/withholding cases. Missing higher-priority
evidence must not silently fall back to a different donor or payer.

Kani checks ten concrete company metadata boundary cases and all 128 ASCII
suffixes of `0000000?`. The suffix proof requires alphanumeric input and exact
uppercase normalization. The other cases cover missing/company/person/organisation/
unrecognized types, length, punctuation, digit presence, whitespace and case.
These are limited boundary proofs, not a proof of the entire resolver. General
symbolic string and full-resolver attempts timed out during symbolic execution.

Keep all default Kani safety and unwinding checks enabled. The runner bounds
ordinary loops at 12 and `memcmp.0` at 64. An insufficient bound, unsupported
operation, timeout, missing result, or failed assertion is a failed verification
run. Raise a bound only after inspecting the failure; never suppress its check.

The parser currently does **not** copy a surrounding declaration's donor type or
company number into nested payments. The existing source-level example is
[keeps_multiple_funding_entries_with_their_own_details](../../../exposed/src/lib/outbound/parliament_api/declarations/tests.rs).
The separate parent rule selects the unique same-member, immediate-parent root
payer only when `is_ultimate_payer_different` is `Some(false)`. This skill checks that selection after source
projection. A native source-field check enumerates 64 combinations of parent/child
type and identifier presence, one or two nesting levels, and donor/payment groups.
It requires each scope to retain its own values, including missing values. These
are exhaustive fixture tests, not Kani proofs. Full version selection and cleaning
remain outside coverage. Do not reinterpret missing fields as an inheritance rule.

## Source coverage

Last reviewed source commit: `a01264dc7f76015d208e6137a858561d3453e4d5`.

- [identity.rs](../../../exposed/src/lib/domain/models/declaration_resolution/identity.rs):
  `company`, `kind`, `classify`, `expanded_edges`, `Components` and `resolve`.
  Kani directly checks `company`; the native graph checks execute the wider path.
- [attribution.rs](../../../exposed/src/lib/domain/models/declaration_resolution/attribution.rs):
  `attribute`, `local_selection`, `parent_selection` and `parent_cycle`, covered by
  the native suite.
- [input.rs](../../../exposed/src/lib/domain/models/declaration_resolution/input.rs):
  canonical input types and `is_root`. Input constructors and validation are not
  executed. Fixture IDs are short ordered tokens. Scope and role metadata follows
  the specified cases, but source-ID spelling is outside coverage.
- [declaration_resolution.rs](../../../exposed/src/lib/domain/models/declaration_resolution.rs),
  [declaration_cleaning.rs](../../../exposed/src/lib/domain/models/declaration_cleaning.rs),
  [declaration_ingestion.rs](../../../exposed/src/lib/domain/models/declaration_ingestion.rs),
  [parliament_member.rs](../../../exposed/src/lib/domain/models/parliament_member.rs), and
  [entity_ingestion.rs](../../../exposed/src/lib/domain/models/entity_ingestion.rs)
  supply canonical types and constants.
- [declaration_source.rs](../../../exposed/src/lib/outbound/declaration_source.rs):
  `funding_entries`, `parse_funding_entry`, `has_funding_anchor`, and their canonical
  decoding types. The native scope checks execute these exact source items with
  `CapturedFundingEntry` and its original methods. Only their JSON error type is
  narrowed to `serde_json::Error`; the checked functions emit only JSON errors.

The [build script](kani/build.rs) selects these declarations by syntax rather
than copying their fields or enum variants. It removes serialization/diagnostic
attributes and adjusts module visibility. Two unreachable external error payloads
have empty stand-ins. It includes the unchanged identity and attribution modules;
matching logic, collections, and string operations are not stubbed.

Before updating the source commit, inspect changes in every covered owner and
confirm that the fixtures and assertions still describe its policy. Run both
suites and the mutation checks. Preserve unexpected failures as findings with
their inputs. Distinguish an actual policy violation from a bad fixture or tool
limit. Do not weaken an assertion or change product behavior to obtain a pass.
Use `$pstack:maintain-formal-verification-skill` for a broader coverage update.

## Evidence and diagnosis

Each run prints a new directory under `target/funder-identity-evidence`.
It retains source hashes, revision/dirty status, commands, logs, Kani's machine
results, mutation assertions, and changed source copies. An interrupted run or
missing final summary is incomplete. Keep that evidence when reporting a failure.

Use `--harness identity::every_ascii_suffix_obeys_company_format` to isolate that
Kani proof; the native suite still runs. `--timeout SECONDS` changes the per-harness
time limit, not the input domain. Report native results and Kani results separately.
Neither suite verifies the database, HTTP serialization, raw-source completeness,
financial totals, arbitrary Unicode input, or later funder display grouping.
