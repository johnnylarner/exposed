# Weldrs comparison prototype

This throwaway instrument compares weldrs 0.2.2 with the original Splink 4.0.17 and DuckDB 1.4.4 pipeline. It is a standalone crate outside the application workspace. It does not change production resolution.

## Findings

The full retained capture contains 7,986 funder observations and 8,595 payments. The canonical application input produces 5,821 profiles and 117,575 candidate pairs. Both scorers feed the same application identity and attribution source files.

The original Splink run and every weldrs variant produce identical observation identities, identity bases, pair dispositions, pair reasons, and payment attributions on this capture. The policy expands the candidates into 229,886 observation-pair decisions. Of these, 24,611 are accepted, 52,205 are rejected, and 153,070 require review. There are 8,555 selected payment attributions and 40 unavailable attributions.

Score parity requires deliberate configuration.

| Distance implementation and adapter | Different scores on the full capture |
| --- | ---: |
| Default scalar distance and stock null handling | 15,449 |
| Default scalar distance with neutral missing addresses | 88 |
| Default scalar distance with neutral missing addresses and byte encoding | 9 |
| SIMD byte distance and stock null handling | 15,362 |
| SIMD byte distance with neutral missing addresses | 0 |

The last variant's maximum absolute probability difference is approximately `1.39e-16`. The comparison tolerance is `1e-12 * max(abs(reference_probability), 1e-12)`. This is numerical parity on the observed inputs. It does not establish accuracy against labelled entity matches. The existing model remains uncalibrated.

Weldrs' stock null level requires both addresses to be missing. Splink treats either missing address as neutral evidence. The prototype changes the address gamma to `-1` before calling weldrs' scorer.

Weldrs' default distance uses Unicode characters. The existing DuckDB function uses UTF-8 bytes. For `伊藤` and `伊東`, DuckDB returns distance 3, while scalar weldrs accepts distance at most 2. Weldrs' `simd` feature uses byte distance and agrees with DuckDB on this example.

The scalar implementation also accepts `andrew lichnowski` and `andy lichnowski` at distance at most 2. DuckDB returns 3. The scalar counterexample fixture reproduces the discrepancy without the capture. The public library example in `examples/distance.rs` isolates the behavior. An independent source reproduction identified a stale cell immediately left of the moving dynamic-programming band. The SIMD implementation rejects this pair. [Issue #38](https://github.com/johnnylarner/exposed/issues/38) tracks this dependency defect.

I recommend implementing and validating a production weldrs adapter using its SIMD distance implementation and explicit missing-address handling. Weldrs would own the comparison and probability algorithms. The application would retain its model parameters, blocking rules, identity policy, and attribution policy. That removes generated Splink SQL and Python from inference.

## Adapter responsibilities

All variants use the same blocking adapter. Weldrs 0.2.2 exposes equijoins rather than the current arbitrary SQL predicates. The prototype derives prefix and surname columns, expands blocking-key tokens, applies the left-surname exclusion and anchor filter, and removes duplicate pairs. It uses weldrs' public candidate generator for each join. The asymmetric surname exclusion, Unicode prefixes, empty regex matches, escaped tokens, repeated keys, and overlapping rules match the existing model.

The prototype checks the candidate budget after candidate materialization and before comparison and scoring. Canonical policy checks the expanded observation budget. A production adapter needs an allocation-safe budget strategy before large joins. This prototype does not establish peak memory, throughput, build portability, or a speedup. It does not test model training or dashboards.

The comparison definitions describe the current frozen model. The code reads its prior and m/u probabilities, but does not translate arbitrary Splink SQL comparisons into weldrs predicates.

`build.rs` copies canonical source files into Cargo's output directory so the standalone crate can access crate-private constructors. The source policy itself is unchanged. This technique belongs only to the experiment. A production adapter should implement the existing scorer port.

## Artifacts

`reference.json` contains freshly generated original Splink predictions for 43 portable cases. These include the existing 22 score fixtures, one distance counterexample, and 20 canonical policy cases. Policy cases cover statistical links, donor rules, aliases, typed unions, identity conflicts, withheld names, repeated-profile budget refusal, duplicate payment occurrences, parent flags, explicit ultimate payers, and parent cycles.

`result-scalar.json` and `result-simd.json` record portable comparisons. `summary.json` records aggregate fixture and capture results. Each run compares candidates, gamma levels, probabilities, direct and lazy scoring, and reversed input order. Policy equality excludes probabilities, gamma levels, and manifests. The score comparison reports those numerical differences separately.

The full input and pair reports stay in `/tmp/exposed-weldrs-comparison-20261009/`. The retained input is ingestion `01a120ef-db40-7183-b9b1-70b941ae6f52`. Its input digests are recorded in `summary.json`. The repository contains aggregate capture results rather than a second copy of the declaration dataset.

## Run the native comparison

From the repository root, run the portable reference cases.

```sh
cargo run --locked --manifest-path prototypes/weldrs-comparison/Cargo.toml -- compare prototypes/weldrs-comparison/reference.json /tmp/result-scalar.json
cargo run --locked --manifest-path prototypes/weldrs-comparison/Cargo.toml --features splink-simd -- compare prototypes/weldrs-comparison/reference.json /tmp/result-simd.json
```

The comparison executable does not need Python. Python is used only to regenerate the original Splink baseline or extract retained Parquet files for this experiment. `summarize.py` also uses Python's standard library for optional report summaries.

To isolate the dependency distance behavior, run the example with each feature selection.

```sh
cargo run --locked --manifest-path prototypes/weldrs-comparison/Cargo.toml --example distance
cargo run --locked --manifest-path prototypes/weldrs-comparison/Cargo.toml --example distance --features splink-simd
```

## Regenerate the original baseline

Use an environment with the pinned versions in `resolution/requirements.txt`. `splink_reference.py` calls the existing `resolution/generate.py` prediction function. It does not implement a second reference scorer.

```sh
cargo run --locked --manifest-path prototypes/weldrs-comparison/Cargo.toml -- export /tmp/inputs.json
python prototypes/weldrs-comparison/splink_reference.py /tmp/inputs.json /tmp/reference.json
```

For the full capture, extract cleaned inputs first. Use the same cleaned JSON for export and comparison.

```sh
python prototypes/weldrs-comparison/extract_cleaned.py /path/to/cleaned/declarations /tmp/cleaned.json
cargo run --locked --manifest-path prototypes/weldrs-comparison/Cargo.toml -- export /tmp/full-inputs.json /tmp/cleaned.json
python prototypes/weldrs-comparison/splink_reference.py /tmp/full-inputs.json /tmp/full-reference.json
cargo run --locked --manifest-path prototypes/weldrs-comparison/Cargo.toml --features splink-simd -- compare /tmp/full-reference.json /tmp/full-result.json /tmp/cleaned.json
python prototypes/weldrs-comparison/summarize.py /tmp/full-result.json
```

The executable rejects a baseline whose policy profiles differ from the current canonical input.
