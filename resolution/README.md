# Run declaration resolution

Use Python 3.10 or later. Install the statistical runtime before invoking the CLI. From the repository root, create a Python environment and install the pinned package dependencies:

```sh
python3 -m venv resolution/.venv
resolution/.venv/bin/python -m pip install ./resolution
```

The package pins Splink 4.0.17 and DuckDB 1.4.4. The repository worker script calls the packaged scorer. No packages are installed at resolution runtime. The bundled frozen model supplies every comparison parameter. Its probabilities are computed but uncalibrated.

Configure an explicit interpreter and the ingestion data directory:

```yaml
data_dir: data
resolution_python: resolution/.venv/bin/python
resolution_worker: resolution/worker.py
candidate_budget: 1000000
```

Paths are relative to the invocation directory. `resolution_worker` defaults to `resolution/worker.py`, so invoke from the repository root or supply an absolute path. The budget applies to profile candidates and expanded source-observation pairs. Increase it explicitly when the command refuses a larger candidate set.

Resolve an existing cleaned run:

```sh
cargo run --package exposed --bin exposed -- data declarations resolve exposed/config/declaration-resolution-dev.yaml --ingestion-key UUID
```

The command prints the output path and counts. The bundle contains observation resolutions, payment attributions, candidate decisions, and a manifest. It refuses an existing bundle. Older cleaned schemas require cleaning retained raw capture into a fresh run. Raw files are not needed after the current cleaner has produced the checked pair.

Run worker integration tests against actual Splink:

```sh
resolution/.venv/bin/python -m unittest discover -s resolution/tests -v
```

Run the real CLI test with a build environment that supplies `DATABASE_URL` or the SQLx query cache for existing database adapters:

```sh
EXPOSED_RESOLUTION_PYTHON="$PWD/resolution/.venv/bin/python" cargo test --package exposed --test declaration_resolution_cli -- --ignored
```

This test builds and invokes the current CLI, cleans retained raw evidence, and resolves two charity observations with statistical support and four Gary Lubner donor observations with missing or disagreeing addresses. It checks the actual names in each identity, the real Splink scores, and refusal to overwrite the result. It removes raw evidence before resolution to verify that the resolver reads only cleaned inputs.

Policy `funder-resolution-v2` accepts donor-to-donor links with the same usable normalized name even when addresses are missing or disagree and the real probability is below 0.999. This deliberately accepts same-name false positives. Other role combinations require exact normalized name, exact usable numbered-street address, and probability at least 0.999. Statistical evidence takes precedence when available. Compatible company and entity-kind evidence is required for every link. Fuzzy names stay review candidates. Each named singleton receives a separate provisional run-local identity. See the [resolution policy](../docs/funder-resolution-design.md) for attribution and repeat behavior.
