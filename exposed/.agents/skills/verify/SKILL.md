---
name: verify
description: Verify declaration cleaning and funder resolution through the real CLI and saved Parquet evidence. Use after changes to funding roles, names, attribution, identity policy, Splink scoring, or publication.
---

# Verify Exposed declarations

Read the [feature map](features/README.md), then the affected feature files. This skill drives `exposed data declarations clean` and `resolve`. The [frontend skill](../../../../frontend/.agents/skills/verify/SKILL.md) covers search. Fetch and database import are outside this proof.

Run from the repository root in a dedicated worktree. Requirements are Bash, Python 3.10+ with venv and pip, Docker, and the repository's Rust toolchain. Launch installs PyArrow and the pinned resolution package into the run's own virtual environment. No application credentials or Parliament capture are needed.

## Launch

Keep the absolute evidence path. Use a fresh directory for every attempt, including retries after failure.

```bash
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
test -f .git
mkdir -p target/verification
export VERIFY_EVIDENCE="$(mktemp -d "$PWD/target/verification/declarations.XXXXXX")"
printf '%s\n' "$VERIFY_EVIDENCE"
exposed/.agents/skills/verify/scripts/launch.sh "$VERIFY_EVIDENCE"
```

Launch starts PostgreSQL 18 on a Docker-assigned loopback port, applies the baseline and role search path to that disposable database, and builds `target/debug/exposed`. SQLx needs the schema for compile-time query checks even though cleaning and resolution do not use a database. The helper removes its container and volume on exit, including failed builds. It never migrates an existing database.

Readiness requires exit code 0, `build.sha256`, and a successful Doctor. Inspect `dependencies.log`, `schema.log`, and `build.log` if Launch fails, then run Cleanup. A clean checkout can take several minutes to compile.

Run one build per worktree at a time because Cargo shares `target/debug/exposed`. Drives in separate evidence directories have separate data and can run concurrently after the build. Rebuild after Rust changes. Doctor detects a changed binary, but does not prove that an old binary includes source edits.

## Doctor

These read-only application checks verify the binary checksum recorded by Launch, both commands' help, and the pinned Splink and DuckDB runtime. They write diagnostic evidence only.

```bash
"$VERIFY_EVIDENCE/venv/bin/python" \
	exposed/.agents/skills/verify/scripts/declarations.py doctor "$VERIFY_EVIDENCE"
"$VERIFY_EVIDENCE/venv/bin/python" \
	exposed/.agents/skills/verify/scripts/resolution.py doctor "$VERIFY_EVIDENCE"
```

Require both Doctor messages and exit codes 0. Rerun these whenever the binary or command looks wrong. Help output is a readiness check, not proof of behavior.

## Drive

```bash
"$VERIFY_EVIDENCE/venv/bin/python" \
	exposed/.agents/skills/verify/scripts/declarations.py drive "$VERIFY_EVIDENCE"
```

The helper writes a synthetic captured partition with the production Arrow schema. It invokes the real binary in an isolated working directory using:

```text
exposed data declarations clean config.yaml --ingestion-key 00000000-0000-4000-8000-000000000001
```

The scratch configuration points to `./data`. An empty `.env` prevents discovery of the developer's environment file, and the command environment excludes `DATABASE_URL`. The helper reads both resulting Parquet tables with PyArrow, checks their joins and literal feature values, then repeats the command to verify that existing output is protected.

For identity resolution, run the second drive against the same built binary:

```bash
"$VERIFY_EVIDENCE/venv/bin/python" \
	exposed/.agents/skills/verify/scripts/resolution.py drive "$VERIFY_EVIDENCE"
```

It creates a separate capture fixture, cleans it through the CLI, removes raw input, and resolves the cleaned output with the pinned Splink worker. It checks exact donor-name links across missing and conflicting addresses, statistical charity links, company-number separation, role-restricted links, explicit ultimate-payer attribution, occurrence preservation, and rejected overwrite. See [identity resolution](features/identity-resolution.md) for the cases and limits.

Require exit code 0 and `proof.json` for cleaning; require `resolution/proof.json` for resolution. Read the mapped recipes for individual assertions and limitations. The [name cases](name-cases.json) hold literal cleaning expectations, with each name tested as donor, payer, and ultimate payer. Extend those cases when the intended cleaning rules change.

## Evidence

Evidence stays in the printed absolute `target/verification/declarations.*` directory. The helper retains:

- `revision.txt`, `source.diff`, `source-status.txt`, `build.sha256`, and dependency versions to identify the build and environment.
- `*.command.json`, `*.stdout.txt`, and `*.stderr.txt` to record each command, working directory, exit code, and output.
- `raw-input.parquet` and `name-cases.json` to preserve the fixture and its name expectations.
- `tables/funding_entries.parquet`, `tables/funders.parquet`, and their JSON projections to inspect the resulting rows independently.
- `proof.json` to record completed feature checks. Failed runs have no success proof.
- `resolution/` with its own cases, command logs, copied clean and resolved bundles, JSON row projections, and `proof.json`.
- Build and cleanup logs to identify the resources the run created and removed.

Prove the real CLI path and resulting files together. A successful process exit alone is insufficient. The helpers verify unchanged published bytes after rejected repeats. Neither calls the Rust cleaner directly or inserts cleaned output as a fixture.

The fixture replaces the captured-input boundary only. This proves cleaning behavior, not live acquisition or database import. Launch may download a Docker image, Python packages, and Rust dependencies. This workflow has no dry-run mode. If you add one, observe file and network effects before claiming it is read-only.

## Cleanup

Launch removes its own database. Both drives remove their scratch directories in a `finally` block, including assertion failures. Run this final check after every attempt. It also handles an interrupted helper.

```bash
if test -s "$VERIFY_EVIDENCE/container.id"; then
	VERIFY_CONTAINER="$(cat "$VERIFY_EVIDENCE/container.id")"
	if docker container inspect "$VERIFY_CONTAINER" > /dev/null 2>&1; then
		test "$(docker inspect -f '{{index .Config.Labels "exposed.verification"}}' "$VERIFY_CONTAINER")" = declarations
		docker logs "$VERIFY_CONTAINER" > "$VERIFY_EVIDENCE/postgres.log" 2>&1
		docker rm -f -v "$VERIFY_CONTAINER" >> "$VERIFY_EVIDENCE/build-cleanup.log"
	fi
	test -z "$(docker ps -aq --no-trunc --filter "id=$VERIFY_CONTAINER")"
fi
"$VERIFY_EVIDENCE/venv/bin/python" - "$VERIFY_EVIDENCE" <<'PY'
import shutil
import sys
from pathlib import Path
evidence = Path(sys.argv[1]).resolve(strict=True)
assert evidence.parent.name == "verification" and evidence.name.startswith("declarations.")
assert (evidence / "revision.txt").is_file()
scratch = evidence / "scratch"
resolution_scratch = evidence / "resolution/scratch"
for path in (scratch, resolution_scratch):
    if path.exists():
        shutil.rmtree(path)
    assert not path.exists()
print(f"Evidence retained at {evidence}")
PY
```

If dependency installation failed before the virtual environment became usable, use `python3` for this cleanup block. Never remove the evidence directory or stop containers by process name. The virtual environment and Cargo cache can remain for inspection and future builds.

After a successful run, confirm both proof files, raw fixtures, and the cleaned and resolved Parquet bundles still exist. Preserve logs after failed runs too.

## Helpers

- `scripts/launch.sh EVIDENCE_DIRECTORY` builds the CLI and removes its temporary database. Invoke it as shown in Launch.
- `scripts/declarations.py doctor EVIDENCE_DIRECTORY` checks the recorded binary. Invoke it with the run's Python as shown in Doctor.
- `scripts/declarations.py drive EVIDENCE_DIRECTORY` writes fixtures, runs the CLI, reads the results, and removes scratch state. Invoke it as shown in Drive.
- `scripts/resolution.py doctor EVIDENCE_DIRECTORY` checks the resolver and pinned worker runtime.
- `scripts/resolution.py drive EVIDENCE_DIRECTORY` runs clean → resolve, checks identity and attribution, and retains evidence under `resolution/`.

Use `/maintain-verification-skill` to keep this map and its checks current as cleaning behavior changes.
