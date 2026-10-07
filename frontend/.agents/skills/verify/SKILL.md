---
name: verify
description: Drive the Exposed Svelte search UI against its Rust API and PostgreSQL database. Use after changes to search, result display, keyboard controls, URL state, or frontend/API integration to capture browser and database evidence.
---

# Verify Exposed search

Read the [feature map](features/README.md), then the affected feature files. The primary user path is the search page. The Rust `/search` endpoint supports it. Data ingestion commands and the legacy Python importer are separate workflows described in the repository README.

Run commands from the repository root in a dedicated Git worktree. Use Bash for the shell blocks. Requirements are Docker Compose 2.24.4 or newer, SQLx CLI 0.9.0, Python 3, Node.js 24+, npm, and curl. There is no application login.

## Launch

Each run uses a unique Compose project and a [port override](compose.yaml). Docker assigns free loopback ports when it starts the containers. The run has its own database and volumes and leaves the worktree's environment files alone. Drive one session per worktree. Keep one Bash session, or source the saved `$VERIFY_EVIDENCE/run.env` in each new shell.

```bash
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
test -f .git
mkdir -p target/verification
export VERIFY_EVIDENCE="$(mktemp -d "$PWD/target/verification/run.XXXXXX")"
export VERIFY_PROJECT="$(python3 -c 'import uuid; print("exposed-verify-" + uuid.uuid4().hex[:12])')"
export VERIFY_COMPOSE="$PWD/frontend/.agents/skills/verify/compose.yaml"
git rev-parse HEAD > "$VERIFY_EVIDENCE/revision.txt"
git diff HEAD > "$VERIFY_EVIDENCE/source.diff"
git status --short > "$VERIFY_EVIDENCE/source-status.txt"
printf 'export VERIFY_PROJECT=%q\nexport VERIFY_COMPOSE=%q\nexport VERIFY_EVIDENCE=%q\n' \
	"$VERIFY_PROJECT" "$VERIFY_COMPOSE" "$VERIFY_EVIDENCE" > "$VERIFY_EVIDENCE/run.env"
printf '%s\n' "$PWD" > "$VERIFY_EVIDENCE/worktree.txt"
docker compose -f compose.yaml -f "$VERIFY_COMPOSE" \
	-p "$VERIFY_PROJECT" config > "$VERIFY_EVIDENCE/compose-config.yaml"
docker compose -f compose.yaml -f "$VERIFY_COMPOSE" \
	-p "$VERIFY_PROJECT" up -d --wait postgres
VERIFY_DATABASE_ADDRESS="$(docker compose -f compose.yaml -f "$VERIFY_COMPOSE" \
	-p "$VERIFY_PROJECT" port postgres 5432)"
export DATABASE_URL="postgresql://exposed:exposed_local_dev@$VERIFY_DATABASE_ADDRESS/exposed?sslmode=disable"
sqlx migrate run > "$VERIFY_EVIDENCE/migrations.log" 2>&1
```

Seed the empty database with the repository's existing test fixtures. These are synthetic test records, not a current Parliament import. `John` returns two MPs, `McDonald's` returns a company, and `Aaron Banks` returns an individual funder. `HSBC` has no match in this fixture set.

```bash
test "$(docker compose -f compose.yaml -f "$VERIFY_COMPOSE" \
	-p "$VERIFY_PROJECT" exec -T postgres \
	psql -X -U exposed -d exposed -Atc 'SELECT count(*) FROM exposed.members')" = 0
docker compose -f compose.yaml -f "$VERIFY_COMPOSE" \
	-p "$VERIFY_PROJECT" exec -T postgres \
	psql -X -v ON_ERROR_STOP=1 -U exposed -d exposed \
	< db/fixtures/add_members.sql > "$VERIFY_EVIDENCE/seed.log"
docker compose -f compose.yaml -f "$VERIFY_COMPOSE" \
	-p "$VERIFY_PROJECT" exec -T postgres \
	psql -X -v ON_ERROR_STOP=1 -U exposed -d exposed \
	< db/fixtures/add_declarations_and_funding_entries.sql >> "$VERIFY_EVIDENCE/seed.log"
docker compose -f compose.yaml -f "$VERIFY_COMPOSE" \
	-p "$VERIFY_PROJECT" up -d --build exposed frontend \
	> "$VERIFY_EVIDENCE/launch.log" 2>&1
export VERIFY_BASE_URL="http://$(docker compose -f compose.yaml -f "$VERIFY_COMPOSE" \
	-p "$VERIFY_PROJECT" port frontend 5173)"
export VERIFY_API_URL="http://$(docker compose -f compose.yaml -f "$VERIFY_COMPOSE" \
	-p "$VERIFY_PROJECT" port exposed 6999)"
printf 'export VERIFY_BASE_URL=%q\nexport VERIFY_API_URL=%q\n' \
	"$VERIFY_BASE_URL" "$VERIFY_API_URL" >> "$VERIFY_EVIDENCE/run.env"
npm --prefix frontend ci --no-audit --no-fund
(cd frontend && PLAYWRIGHT_BROWSERS_PATH=0 npx playwright install chromium)
curl --fail --silent --show-error --retry 90 --retry-delay 2 \
	--retry-all-errors --max-time 3 \
	"$VERIFY_BASE_URL/api/search?term=John&max_entries=10&strictness=1" \
	> "$VERIFY_EVIDENCE/ready.json"
```

Readiness requires an HTTP 200 from that proxied search with both fixture MPs. The Rust log says `Listening on http://localhost:6999` inside the container. A frontend HTML response alone does not establish API readiness. The first Rust build can take longer than the frontend startup. If the bounded retry fails, inspect the Compose logs, run Doctor, and clean up before retrying.

The current checkout contains a later migration that removes term tables as well as the initial baseline. Apply the checked-in migrations as they are. Do not change migration history to run this skill. See `db/README.md` for schema work.

## Doctor

Run this read-only check before driving and whenever an instance looks wrong. It checks container ownership, the source mount, running state, the Vite proxy, and the expected database fixtures. A failed check means the instance is not ready to drive.

```bash
(
	set -euo pipefail
	for service in postgres exposed frontend; do
		container="$(docker compose -f compose.yaml -f "$VERIFY_COMPOSE" \
			-p "$VERIFY_PROJECT" ps -q "$service")"
		test -n "$container"
		test "$(docker inspect -f '{{.State.Running}}' "$container")" = true
		test "$(docker inspect -f '{{index .Config.Labels "com.docker.compose.project.working_dir"}}' "$container")" = "$PWD"
		if test "$service" = exposed; then
			test "$(docker inspect -f '{{range .Mounts}}{{if eq .Destination "/app"}}{{.Source}}{{end}}{{end}}' "$container")" = "$PWD"
		fi
	done
	curl --fail --silent --show-error --max-time 5 \
		"$VERIFY_BASE_URL/api/search?term=John&max_entries=10&strictness=1" |
		python3 -c 'import json,sys; rows=json.load(sys.stdin)["entities"]; assert len(rows)==2 and {r["name"] for r in rows}=={"John McDonnell","John Humphries"} and all(r["kind"]=="MP" for r in rows); print("Doctor passed: owned stack, real API, expected MPs")'
) | tee "$VERIFY_EVIDENCE/doctor.log"
```

Source edits are bind-mounted. Before proving a Rust change, require its rebuild to finish in the `exposed` log and then rerun Doctor. Doctor is a readiness check, not proof of the changed behavior.

## Drive

Capture the database before browser actions. PostgreSQL 18's fixed dump restriction key makes the two local snapshots comparable.

```bash
docker compose -f compose.yaml -f "$VERIFY_COMPOSE" \
	-p "$VERIFY_PROJECT" exec -T postgres \
	pg_dump -U exposed -d exposed --data-only --schema=exposed \
	--no-owner --restrict-key=ExposedVerification \
	> "$VERIFY_EVIDENCE/database-before.sql"
PLAYWRIGHT_BROWSERS_PATH=0 \
	frontend/.agents/skills/verify/scripts/search.mjs \
	"$VERIFY_BASE_URL" "$VERIFY_EVIDENCE/search" \
	> "$VERIFY_EVIDENCE/search.log" 2>&1
docker compose -f compose.yaml -f "$VERIFY_COMPOSE" \
	-p "$VERIFY_PROJECT" exec -T postgres \
	pg_dump -U exposed -d exposed --data-only --schema=exposed \
	--no-owner --restrict-key=ExposedVerification \
	> "$VERIFY_EVIDENCE/database-after.sql"
cmp "$VERIFY_EVIDENCE/database-before.sql" "$VERIFY_EVIDENCE/database-after.sql"
```

The helper uses the installed Playwright package and its Chromium browser. It launches a fresh browser context, exercises the short-query gate, focuses search with `/`, types `John`, compares visible names with the real response, and clears with the button. It closes its browser even after an assertion fails. Exit code 0 and `search/proof.json` establish this path. Other mapped entry points still need their own actions.

For another feature, copy the helper to a unique `.mjs` file beside it so Node can resolve `frontend/node_modules`. Adapt its actions using the matching feature recipe, and pass a new evidence subdirectory. Keep its trace, failure capture, and browser cleanup. Remove that temporary script in Cleanup. Do not register `page.route` handlers for live proof.

The existing `frontend/tests/search.spec.ts` suite controls the API with fabricated responses. Use it to supplement cancellation and error-edge checks, and label its output as frontend-only. It is not evidence that Rust or PostgreSQL worked. Its fixed port 4173 and server reuse also need care. Run it with `CI=1` and only when port 4173 is free.

## Evidence

Keep all proof under the printed absolute `$VERIFY_EVIDENCE` path, inside the worktree's ignored `target/verification/`. Do not use `frontend/test-results` for durable proof because another Playwright test run can clear it.

The helper preserves `trace.zip`, before/result/cleared screenshots, ARIA snapshots, the real `response.json`, and a `proof.json` with the feature ID and entry points. Failures retain `failure.txt` and a screenshot when a page exists. The trace records actions and network responses, not only the final page.

Pair each claimed entry point with its action and observable result. Compare real API ordering and spelling with the UI. Compare database snapshots for search's absence of writes. For future mutating features, inspect the stored row or file after the user action. Mock only an external system behind an existing production boundary, and identify that limitation in the proof. Do not substitute internal state setters or test-only endpoints for the user path.

This workflow has no dry-run mode. It writes fixtures to a disposable database, starts local containers, and may download images, dependencies, and Chromium. It does not fetch Parliament data. If a future command claims to be a dry-run, check its files, requests, and Git refs before and after instead of trusting the flag.

## Cleanup

Run cleanup after every attempt, including failed launches or browser assertions. The saved run environment identifies this project even after a shell restart. Set `VERIFY_EVIDENCE` to the recorded run directory when resuming. Do not remove the evidence directory.

```bash
test "$(cat "$VERIFY_EVIDENCE/worktree.txt")" = "$PWD"
source "$VERIFY_EVIDENCE/run.env"
docker compose -f compose.yaml -f "$VERIFY_COMPOSE" \
	-p "$VERIFY_PROJECT" logs --no-color \
	> "$VERIFY_EVIDENCE/compose.log" 2>&1
docker compose -f compose.yaml -f "$VERIFY_COMPOSE" \
	-p "$VERIFY_PROJECT" down --volumes \
	> "$VERIFY_EVIDENCE/cleanup.log" 2>&1
test -z "$(docker ps -aq --filter "label=com.docker.compose.project=$VERIFY_PROJECT")"
test -s "$VERIFY_EVIDENCE/cleanup.log"
printf 'Evidence retained at %s\n' "$VERIFY_EVIDENCE"
```

`down --volumes` removes only this disposable Compose project's instances, database, and dependency volumes. Host dependencies and Docker image caches can remain. Remove any temporary driver copy you created by its exact path. Never kill by process name or stop another project's containers. After a successful proof, confirm `search/proof.json` and `search/trace.zip` still exist after cleanup. After a failed attempt, preserve its logs and any failure artifacts.

## Helpers

- Execute `scripts/search.mjs` as shown in Drive. Its arguments are the owned frontend's loopback URL and a new evidence subdirectory. It requires the fixture database.
- Use `compose.yaml` from this skill as the second Compose file, as shown in Launch. Its `!override` port lists replace the root file's fixed host ports and require Compose 2.24.4 or newer.
- Open a saved trace with `cd frontend && npx playwright show-trace "$VERIFY_EVIDENCE/search/trace.zip"`.

Use `/maintain-verification-skill` when app changes require updates to these recipes or the feature map.
