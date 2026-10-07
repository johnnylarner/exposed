---
name: verify
description: Verify the Exposed declarations CLI by running offline cleaning and inspecting funding_entries.parquet and funders.parquet. Use after changes to funder roles, name features, source replay, or Parquet publication.
---

# Verify Exposed declaration cleaning

Read the [feature map](features/README.md), then the affected feature files. This skill drives `exposed data declarations clean`. The [frontend skill](../../../../frontend/.agents/skills/verify/SKILL.md) covers search. Fetch and database import are outside this proof.

Run from the repository root in a dedicated worktree. Requirements are Bash, Python 3 with venv and pip, Docker, and the repository's Rust toolchain. Launch installs PyArrow into the run's own virtual environment. No application credentials or Parliament capture are needed.

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

Launch starts PostgreSQL 18 on a Docker-assigned loopback port, applies the baseline to that disposable database, and builds `target/debug/exposed`. SQLx needs the schema for compile-time query checks even though cleaning does not use a database. The helper removes its container and volume on exit, including failed builds. It never migrates an existing database.

Readiness requires exit code 0, `build.sha256`, and a successful Doctor. Inspect `dependencies.log`, `schema.log`, and `build.log` if Launch fails, then run Cleanup. A clean checkout can take several minutes to compile.

Run one build per worktree at a time because Cargo shares `target/debug/exposed`. Drives in separate evidence directories have separate data and can run concurrently after the build. Rebuild after Rust changes. Doctor detects a changed binary, but does not prove that an old binary includes source edits.

## Doctor

This read-only application check verifies the binary checksum recorded by Launch and the actual cleaning command's help. It writes diagnostic evidence only.

```bash
"$VERIFY_EVIDENCE/venv/bin/python" \
	exposed/.agents/skills/verify/scripts/declarations.py doctor "$VERIFY_EVIDENCE"
```

Require `Doctor passed` and exit code 0. Rerun this whenever the binary or command looks wrong. The help output is a readiness check, not proof of cleaning.

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

Require exit code 0 and `proof.json`. The same drive covers all three mapped features. Read their recipes for individual assertions and limitations. The [name cases](name-cases.json) hold literal expectations, with each name tested as donor, payer, and ultimate payer. Extend those cases when the intended cleaning rules change.

## Evidence

Evidence stays in the printed absolute `target/verification/declarations.*` directory. The helper retains:

- `revision.txt`, `source.diff`, `source-status.txt`, `build.sha256`, and dependency versions to identify the build and environment.
- `*.command.json`, `*.stdout.txt`, and `*.stderr.txt` to record each command, working directory, exit code, and output.
- `raw-input.parquet` and `name-cases.json` to preserve the fixture and its name expectations.
- `tables/funding_entries.parquet`, `tables/funders.parquet`, and their JSON projections to inspect the resulting rows independently.
- `proof.json` to record completed feature checks. Failed runs have no success proof.
- Build and cleanup logs to identify the resources the run created and removed.

Prove the real CLI path and resulting files together. A successful process exit alone is insufficient. The helper verifies unchanged raw bytes and unchanged published bytes after the rejected repeat. It never calls the Rust cleaner directly or inserts cleaned output as a fixture.

The fixture replaces the captured-input boundary only. This proves cleaning behavior, not live acquisition or database import. Launch may download a Docker image, Python packages, and Rust dependencies. This workflow has no dry-run mode. If you add one, observe file and network effects before claiming it is read-only.

## Cleanup

Launch removes its own database. Drive removes its scratch directory in a `finally` block, including assertion failures. Run this final check after every attempt. It also handles an interrupted helper.

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
if scratch.exists():
    shutil.rmtree(scratch)
assert not scratch.exists()
print(f"Evidence retained at {evidence}")
PY
```

If dependency installation failed before the virtual environment became usable, use `python3` for this cleanup block. Never remove the evidence directory or stop containers by process name. The virtual environment and Cargo cache can remain for inspection and future builds.

After a successful run, confirm that `proof.json`, `raw-input.parquet`, and both files under `tables/` still exist. Preserve logs after failed runs too.

## Helpers

- `scripts/launch.sh EVIDENCE_DIRECTORY` builds the CLI and removes its temporary database. Invoke it as shown in Launch.
- `scripts/declarations.py doctor EVIDENCE_DIRECTORY` checks the recorded binary. Invoke it with the run's Python as shown in Doctor.
- `scripts/declarations.py drive EVIDENCE_DIRECTORY` writes fixtures, runs the CLI, reads the results, and removes scratch state. Invoke it as shown in Drive.

Use `/maintain-verification-skill` to keep this map and its checks current as cleaning behavior changes.
