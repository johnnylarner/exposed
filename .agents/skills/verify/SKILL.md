---
name: verify
description: Verify Exposed search results and recorded funder names through the real HTTP API against the frozen search regression database. Use after changing search queries, ranking, alias matching, or API serialization.
---

# Verify Exposed search

Run from the repository root. Read [the feature map](features/README.md).
The default strictness for this regression check is `0.8` for every search.
This check freezes search behavior over persisted identities. It does not rerun
declaration resolution or verify frontend rendering.

The launch driver and all five HTTP comparisons passed against the frozen
snapshot while the development API occupied port 6999.

## Launch

Require Python 3, Cargo, and the existing Compose PostgreSQL service.
Check that the snapshot exists without changing it:

```sh
docker compose exec -T postgres psql -X -U exposed -d exposed_search_regression -c 'SELECT current_database()'
```

If the database is absent, report the missing prerequisite. The existing
`scripts/refresh-search-regression-db.sh` replaces the snapshot and requires
stopping source clients. Never refresh it as part of a regression run.

Build the current worktree's API with a read-only database connection:

```sh
export DATABASE_URL='postgresql://exposed:exposed_local_dev@127.0.0.1:55432/exposed_search_regression?sslmode=disable&options=-c%20default_transaction_read_only%3Don'
cargo build --package exposed --bin exposed
```

## Doctor and drive

Run the [launch driver](../../../scripts/verify-search-regression.py) from the
current worktree's repository root. Give it a new evidence directory:

```sh
evidence_dir="/tmp/exposed-search-regression-$(uuidgen)"
python3 scripts/verify-search-regression.py "$evidence_dir"
```

The driver starts `target/debug/exposed` from the current working directory,
checks that its process stays alive, and requires its printed port to differ
from 6999. It temporarily reserves the wildcard address on 6999 to force an
isolated port. On macOS, a wildcard listener can coexist with an existing
address-specific listener, so startup and a response on 6999 do not identify
this run's API.

The driver prints the owned PID and actual base URL, then runs the current
worktree's [HTTP check](../../../scripts/check-search-regression.py).
Separate runs have their own processes, ports, and evidence directories and
share only the read-only snapshot. Do not run during a snapshot refresh.

Require five `PASS` lines and exit code 0. Cases and accepted results live in
`scripts/search-regression.json`. Keep the exact entity order, names,
match provenance, and recorded alias lists. The checker has no baseline-update
mode. Investigate a diff before changing expectations.

## Evidence

Retain `server.log`, `server.txt`, and the `http` directory under the printed `evidence_dir`.
Each case stores the GET URL, complete HTTP response, expected selected field,
and comparison diff. Search compares the entity array with IDs omitted. Funder detail compares the
funder profile with its ID omitted, including its recorded name and aliases.
Detail URLs use IDs from the current search response, never frozen IDs.
Funding totals are outside this check.

Use the real `/search` and `/funders/{UUID}` routes. Do not replace them with
mocks or direct database reads. Report failures with their diff and exit code.
The API connection is read-only and these routes only read stored data.

## Cleanup

The driver terminates and waits for only its own API process after success or
failure. Keep the snapshot and evidence. If failure happens before HTTP capture,
retain the startup log and report the missing response evidence.

## Helpers

`scripts/check-search-regression.py BASE_URL NEW_EVIDENCE_DIRECTORY` needs only
Python's standard library. `--cases PATH` selects a different case file for a
temporary failure probe. It sends GET requests and never writes the database or
accepted baseline. Its evidence directory must not already exist.
