#!/usr/bin/env bash
set -euo pipefail

repo="$(cd "$(dirname "$0")/../../../../.." && pwd)"
evidence="${1:?Usage: launch.sh EVIDENCE_DIRECTORY}"
cd "$repo"
test -f .git
test -d "$evidence"
evidence="$(cd "$evidence" && pwd)"
test ! -e "$evidence/build.sha256"
git rev-parse HEAD > "$evidence/revision.txt"
git diff HEAD > "$evidence/source.diff"
git status --short > "$evidence/source-status.txt"
python3 -m venv "$evidence/venv"
"$evidence/venv/bin/pip" install pyarrow > "$evidence/dependencies.log" 2>&1
"$evidence/venv/bin/pip" freeze > "$evidence/dependencies.txt"

container=""
trap 'if test -n "$container"; then docker logs "$container" > "$evidence/postgres.log" 2>&1; docker rm -f -v "$container" > "$evidence/build-cleanup.log"; fi' EXIT
docker run -d --label exposed.verification=declarations \
	-e POSTGRES_USER=exposed -e POSTGRES_PASSWORD=verification \
	-e POSTGRES_DB=exposed -p 127.0.0.1::5432 postgres:18-alpine \
	> "$evidence/container.id"
container="$(cat "$evidence/container.id")"
for attempt in {1..60}; do
	if docker exec "$container" pg_isready -U exposed -d exposed > /dev/null 2>&1; then
		break
	fi
	sleep 0.5
done
docker exec "$container" pg_isready -U exposed -d exposed
docker exec -i "$container" psql -X -v ON_ERROR_STOP=1 -U exposed -d exposed \
	< db/migrations/20260915000000_initial.up.sql > "$evidence/schema.log" 2>&1
address="$(docker port "$container" 5432/tcp)"
DATABASE_URL="postgresql://exposed:verification@$address/exposed?sslmode=disable" \
	SQLX_OFFLINE=false CARGO_TARGET_DIR="$repo/target" \
	cargo build --locked --package exposed --bin exposed > "$evidence/build.log" 2>&1
shasum -a 256 target/debug/exposed > "$evidence/build.sha256"
printf '%s\n' "$repo/target/debug/exposed" > "$evidence/binary.txt"
printf 'Built CLI. Evidence: %s\n' "$evidence"
