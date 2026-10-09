# Run declaration resolution

The CLI scores candidates in process with the SIMD implementation in `weldrs` 0.2.2. No Python environment, worker script, or package installation is required. The bundled `funder-frozen-v2` model supplies every comparison parameter. Its probabilities are computed but uncalibrated. This runtime migration preserves policy `funder-resolution-v3` and makes no accuracy claim.

Configure the ingestion data directory and candidate budget:

```yaml
data_dir: data
candidate_budget: 1000000
```

The data directory is relative to the invocation directory. Candidate generation uses posting indexes for blocking keys. The budget bounds profile candidates and expanded source-observation pairs. The adapter checks the profile limit during insertion, before it builds scoring frames. Scoring uses batches of at most 4,096 pairs. Increase the budget explicitly when the command refuses a larger candidate set; candidates are never silently truncated.

Resolve an existing cleaned run:

```sh
cargo run --package exposed --bin exposed -- data declarations resolve --config exposed/config/declaration-resolution-dev.yaml --ingestion-key UUID
```

The command prints the output path and counts. The bundle contains observation resolutions, payment attributions, candidate decisions, and a manifest. It refuses an existing bundle. Old member UUID artifacts require a new capture, followed by cleaning and resolution. Raw files are not needed after the current cleaner has produced the checked pair.

Run the real CLI test with a build environment that supplies `DATABASE_URL` or the SQLx query cache for existing database adapters:

```sh
cargo test --package exposed --test declaration_resolution_cli
```

This test builds and invokes the current CLI with `PATH=/nonexistent`, cleans retained raw evidence, and resolves charity observations, Gary Lubner donors with missing or disagreeing addresses, explicit aliases, and typed Unite-family names. It checks identity membership, computed scores, distinct Carlton Club organizations, and refusal to overwrite the result. It removes raw evidence before resolution to verify that the resolver reads only cleaned inputs. The 43 golden cases in `exposed/tests/fixtures/funder-resolution.json` check native comparison levels and probabilities against the captured frozen-model behavior.

Policy `funder-resolution-v3` uses cleaner-extracted aliases and organization-name evidence, punctuation-normalized numbered-street addresses, and typed candidate keys for aliases and `Unite` trade unions. HSBC's CEO-annotated bank name can link to its legal bank name when distinctive organization terms and the numbered-street address agree. Regional and branch spellings with an explicit `Trade Union` kind and the `Unite` root can resolve to one funder identity. Person/organization conflicts and competing company numbers still block merges; ambiguous fuzzy names stay review candidates. Each named singleton receives a separate provisional run-local identity. See the [resolution policy](../docs/funder-resolution-design.md) for attribution and repeat behavior.

The resolved manifest uses schema version 2 for source-only member identities.
Observation and occurrence keys contain numeric Parliament member IDs. Database
member UUIDs never enter resolution artifacts. Loading resolves those source IDs
to the destination database member UUIDs inside the load transaction.
