#!/usr/bin/env bash
set -euo pipefail
skill=$(cd "$(dirname "$0")/.." && pwd)
VERIFY_WORKTREE=$(cd "$skill/../../.." && pwd)
export VERIFY_WORKTREE
if [ "$#" -eq 1 ] && [ "$1" = --help ]; then
  printf 'Usage: run-snapshot-comparison.sh [--production ORIGIN] [--cases PATH]\nLaunch this worktree in isolated snapshot-backed Compose; retain evidence and clean up own project.\n'
  exit 0
fi
args=("$@")
for ((i=0; i<${#args[@]}; i+=2)); do
  case "${args[i]}" in --production|--cases) ;; *) printf 'Expected --production ORIGIN or --cases PATH\n' >&2; exit 2;; esac
  [ $((i+1)) -lt ${#args[@]} ] || exit 2
done
source_container=${VERIFY_SNAPSHOT_CONTAINER:-exposed-postgres-1}
source_database=${VERIFY_SNAPSHOT_DATABASE:-exposed_search_regression}
source_user=${VERIFY_SNAPSHOT_USER:-exposed}
evidence=$(mktemp -d "${TMPDIR:-/tmp}/exposed-snapshot-comparison.XXXXXX")
project="exposed-verify-$(printf '%s' "$VERIFY_WORKTREE" | shasum | cut -c1-8)-$(date +%s)-$$"
compose=(docker compose -p "$project" -f "$skill/compose.yaml")
started_source=false
comparison_finished=false
# shellcheck disable=SC2329 # Invoked by the EXIT trap.
cleanup() {
  code=$?
  if [ "$code" -ne 0 ] && ! "$comparison_finished"; then code=2; fi
  trap - EXIT INT TERM
  "${compose[@]}" logs --no-color > "$evidence/compose.log" 2>&1 || true
  if "${compose[@]}" down --volumes --remove-orphans > "$evidence/cleanup.log" 2>&1; then
    printf 'removed own project %s\n' "$project" >> "$evidence/cleanup.log"
  else
    code=2
  fi
  if "$started_source"; then docker stop "$source_container" >> "$evidence/cleanup.log" 2>&1 || code=2; fi
  printf 'Evidence: %s\n' "$evidence"
  exit "$code"
}
trap 'cleanup' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
printf 'Evidence: %s\nProject: %s\n' "$evidence" "$project"
{
  printf 'worktree=%s\nproject=%s\nsource_container=%s\nsource_database=%s\nstarted_at=%s\n' "$VERIFY_WORKTREE" "$project" "$source_container" "$source_database" "$(date -u +%FT%TZ)"
  git -C "$VERIFY_WORKTREE" rev-parse HEAD
  git -C "$VERIFY_WORKTREE" status --short
  docker inspect "$source_container" --format '{{.Id}} {{.State.Running}}'
} > "$evidence/provenance.txt"
if [ "$(docker inspect "$source_container" --format '{{.State.Running}}')" != true ]; then
  docker start "$source_container" >> "$evidence/source.log"
  started_source=true
fi
for _attempt in $(seq 1 60); do
  if docker exec "$source_container" pg_isready -U "$source_user" -d "$source_database" >> "$evidence/source.log" 2>&1; then break; fi
  sleep 1
done
# pg_dump uses one consistent source snapshot; only the disposable restore is migrated.
docker exec "$source_container" pg_dump -U "$source_user" -d "$source_database" --format=custom --no-owner --no-acl > "$evidence/snapshot.dump"
"${compose[@]}" build > "$evidence/build.log" 2>&1
"${compose[@]}" up -d --wait postgres > "$evidence/start.log" 2>&1
"${compose[@]}" exec -T postgres pg_restore -U exposed -d verification --no-owner --no-acl --exit-on-error < "$evidence/snapshot.dump" > "$evidence/restore.log" 2>&1
"${compose[@]}" run --rm migrate > "$evidence/migrate.log" 2>&1
"${compose[@]}" up -d exposed frontend >> "$evidence/start.log" 2>&1
port=$("${compose[@]}" port frontend 8080)
development="http://$port"
printf 'development=%s\n' "$development" >> "$evidence/provenance.txt"
ready=false
for _attempt in $(seq 1 90); do
  if "${compose[@]}" exec -T exposed exposed healthcheck >> "$evidence/doctor.log" 2>&1 && curl --fail --silent --show-error --connect-timeout 2 --max-time 5 "$development/search/__data.json?q=unite" > "$evidence/doctor.body.json" 2>> "$evidence/doctor.log"; then ready=true; break; fi
  sleep 1
done
"$ready" || { printf 'Doctor failed\n' >&2; exit 2; }
node --input-type=module - "$skill" "$evidence/doctor.body.json" <<'JS'
import { readFile } from 'node:fs/promises';
const {decode,search}=await import(`${process.argv[2]}/scripts/compare-live.mjs`);
search(decode(await readFile(process.argv[3],'utf8')),'unite');
JS
"${compose[@]}" ps --format json >> "$evidence/doctor.log"
"${compose[@]}" exec -T postgres psql -X -U exposed -d verification -c "SELECT datname, application_name, query FROM pg_stat_activity WHERE datname='verification'" >> "$evidence/doctor.log"
set +e
node "$skill/scripts/compare-live.mjs" --development "$development" --evidence "$evidence/http" "$@"
comparison_code=$?
set -e
comparison_finished=true
if [ "$comparison_code" -gt 2 ]; then comparison_code=2; fi
exit "$comparison_code"
