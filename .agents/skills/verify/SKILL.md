---
name: verify
description: Verify Exposed search, recorded funder names, and grouped funders through the real HTTP API. Use after changing search queries, ranking, alias matching, grouping policy, or API serialization.
---

# Verify Exposed search

Run from the repository root. Read [the feature map](features/README.md).
The default strictness for this regression check is `0.8` for every search.
This check freezes search behavior over persisted identities. It does not rerun
declaration resolution or verify frontend rendering.

The five HTTP comparisons passed against the accepted development API. The
snapshot launch, doctor, and cleanup procedure has not yet been executed because
`exposed_search_regression` was absent. Treat that procedure as unverified until
the snapshot is available and the full run succeeds.

## Launch

Require Python 3, Cargo, and the existing Compose PostgreSQL service.
Check that the snapshot exists without changing it:

```sh
docker compose exec -T postgres psql -X -U exposed -d exposed_search_regression -c 'SELECT current_database()'
```

If the database is absent, report the missing prerequisite. The existing
`scripts/refresh-search-regression-db.sh` replaces the snapshot and requires
stopping source clients. Never refresh it as part of a regression run.

Build and start the current worktree's API with a read-only database connection:

```sh
export DATABASE_URL='postgresql://exposed:exposed_local_dev@127.0.0.1:55432/exposed_search_regression?sslmode=disable&options=-c%20default_transaction_read_only%3Don'
cargo build --package exposed --bin exposed
evidence_dir=$(mktemp -d /tmp/exposed-search-regression.XXXXXX)
./target/debug/exposed server >"$evidence_dir/server.log" 2>&1 &
server_pid=$!
```

Wait for `Listening on http://localhost:PORT` in `server.log` and a successful
HTTP response. Use that printed port. The server chooses an available port when
6999 is occupied. Do not reuse the already running development API.
Separate runs have their own processes, ports, and evidence directories and
share only the read-only snapshot. Do not run during a snapshot refresh.

## Doctor

Confirm the process is alive, its connection selects the snapshot, and its
printed port answers a real request:

```sh
kill -0 "$server_pid"
docker compose exec -T postgres psql -X -U exposed -d exposed_search_regression -c "SELECT datname, application_name, client_addr FROM pg_stat_activity WHERE datname = 'exposed_search_regression'"
cat "$evidence_dir/server.log"
curl --fail --get "$base_url/search" --data-urlencode 'term=unite' --data-urlencode 'strictness=0.8' --data-urlencode 'max_entries=10'
```

Set `base_url` to `http://127.0.0.1:PORT` using the startup log before the curl.
If startup fails, capture the log and clean up this run's process before retrying.

## Drive

Run the executable [HTTP check](../../../scripts/check-search-regression.py):

```sh
./scripts/check-search-regression.py "$base_url" "$evidence_dir/http"
```

Require five `PASS` lines and exit code 0. Cases and accepted results live in
`scripts/search-regression.json`. Keep the exact entity order, names,
match provenance, and recorded alias lists. The script has no baseline-update
mode. Investigate a diff before changing expectations.

## Populated funder groups

After changes to alias grouping or reviewed families, run the additional
`loaded_data_grouped_funders_regression` test. It starts the worktree's API and
checks BBC, GB News, and Premier League searches and detail pages against an
isolated copy of the 12 October 2026 development data. This database is separate
from `exposed_search_regression`. Apply the current migrations to the copy, then
retry the latest declaration run already loaded into it using its unchanged raw,
cleaned, and resolved artifacts. Follow the
[stored funder groups deployment procedure](../../../db/README.md#deploy-stored-funder-groups)
with `DATABASE_URL` set to the copy. Require `Run already loaded` before the test.
Loading a new run can replace source data and invalidate the regression fixture.
Do not change the source database or refresh the frozen search snapshot during
verification.

The preferred-name migration creates empty metadata; the exact load retry applies the
compiled reviewed-family names. Expect BBC, GB News, and Premier League in search
and detail names. Member declaration history continues to show recorded source
names.

Set `GROUPED_FUNDER_REGRESSION_DATABASE_URL` to a read-only URL for that copy.
The test also needs the same schema for SQLx compile-time query checks:

```sh
: "${GROUPED_FUNDER_REGRESSION_DATABASE_URL:?set a populated grouping database URL}"
DATABASE_URL="$GROUPED_FUNDER_REGRESSION_DATABASE_URL" \
  cargo test -p exposed --test integration loaded_data_grouped_funders_regression -- --ignored --exact
```

Require one passing test. It checks one result per search, the documented group
member and funding entry counts, recorded names, and both alias and reviewed
family link evidence. If the populated copy is unavailable, report that
prerequisite; the five-case search snapshot does not replace this check.

## Evidence

Retain `server.log` and the `http` directory under the printed `evidence_dir`.
Each case stores the GET URL, complete HTTP response, expected selected field,
and comparison diff. Search compares the entity array with IDs omitted. Funder detail compares the
funder profile with its ID omitted, including its recorded name and aliases.
Detail URLs use IDs from the current search response, never frozen IDs.
Funding totals are outside this check.

Use the real `/search` and `/funders/{UUID}` routes. Do not replace them with
mocks or direct database reads. Report failures with their diff and exit code.
The API connection is read-only and these routes only read stored data.

## Cleanup

```sh
kill "$server_pid"
wait "$server_pid" || true
test -s "$evidence_dir/server.log"
test -s "$evidence_dir/http/unite.response.json"
printf '%s\n' "$evidence_dir"
```

Kill only the process started by this run. Keep the snapshot and evidence.
Run cleanup after failed attempts too. If failure happened before HTTP capture,
retain the startup log and report the missing response evidence.

## Helpers

`scripts/check-search-regression.py BASE_URL NEW_EVIDENCE_DIRECTORY` needs only
Python's standard library. `--cases PATH` selects a different case file for a
temporary failure probe. It sends GET requests and never writes the database or
accepted baseline. Its evidence directory must not already exist.
