#!/usr/bin/env bash
set -euo pipefail

: "${BINARY_IMAGE:?Set BINARY_IMAGE}"
: "${FRONTEND_SERVER_IMAGE:?Set FRONTEND_SERVER_IMAGE}"
: "${MIGRATE_IMAGE:?Set MIGRATE_IMAGE}"
: "${DATA_IMAGE:?Set DATA_IMAGE}"
network="exposed-smoke-$$"
postgres="$network-postgres"
web="$network-web"
work=$(mktemp -d)
cleanup() {
    docker logs "$web" 2>/dev/null || true
    docker rm -f "$web" "$postgres" >/dev/null 2>&1 || true
    docker network rm "$network" >/dev/null 2>&1 || true
    rm -rf "$work"
}
trap cleanup EXIT

docker run --rm "$BINARY_IMAGE" --help
docker run --rm "$DATA_IMAGE" | grep -q 'latest'
printf 'data_dir: /tmp/empty-ingestions\n' > "$work/config.yaml"
docker run --rm --mount "type=bind,src=$work/config.yaml,dst=/config.yaml,readonly" \
    "$DATA_IMAGE" latest --config /config.yaml | jq -e '.ingestion == null'

docker network create "$network" >/dev/null
docker run -d --name "$postgres" --network "$network" \
    -e POSTGRES_PASSWORD=smoke -e POSTGRES_DB=exposed postgres:18 >/dev/null
ready=false
for _ in {1..60}; do
    if docker exec "$postgres" pg_isready -U postgres -d exposed >/dev/null 2>&1; then
        ready=true
        break
    fi
    sleep 1
done
[ "$ready" = true ]
export DATABASE_URL="postgresql://postgres:smoke@$postgres:5432/exposed?options=-csearch_path%3Dexposed,public"
for _ in 1 2; do
    docker run --rm --network "$network" --env DATABASE_URL "$MIGRATE_IMAGE"
done
docker exec "$postgres" psql -U postgres -d exposed -Atc \
    "SELECT count(*) > 0 FROM _sqlx_migrations WHERE success" | grep -qx t

start_web() {
    docker run -d --name "$web" --network "$network" --env DATABASE_URL \
        -p 127.0.0.1::8080 "$FRONTEND_SERVER_IMAGE" >/dev/null
    port=$(docker port "$web" 8080/tcp | awk -F: '{print $NF}')
    origin="http://127.0.0.1:$port"
    ready=false
    for _ in {1..60}; do
        if curl -fsS "$origin/api/search?term=smoketest&max_entries=5" > "$work/search.json"; then
            ready=true
            break
        fi
        sleep 1
    done
    [ "$ready" = true ]
    jq -e '.entities == []' "$work/search.json"
}

start_web
curl -fsS "$origin/" > "$work/index.html"
grep -q '<html' "$work/index.html"
curl -fsS "$origin/members/deep-link" > "$work/deep.html"
cmp "$work/index.html" "$work/deep.html"
asset=$(sed -n 's/.*src="\([^"]*\/assets\/[^" ]*\)".*/\1/p' "$work/index.html" | head -n 1)
[ -n "$asset" ]
curl -fsS "$origin$asset" >/dev/null
[ "$(curl -s -o /dev/null -w '%{http_code}' "$origin/assets/missing.js")" = 404 ]
[ "$(curl -s -o /dev/null -w '%{http_code}' "$origin/api")" = 308 ]
[ "$(docker inspect -f '{{.Config.User}}' "$web")" = 10001:10001 ]
docker stop --time 10 "$web" >/dev/null
[ "$(docker inspect -f '{{.State.ExitCode}}' "$web")" = 0 ]
docker rm "$web" >/dev/null

for child in exposed nginx; do
    start_web
    docker exec "$web" bash -c 'for entry in /proc/[0-9]*/comm; do
        read -r name < "$entry" || continue
        if [ "$name" = "$1" ]; then
            pid=${entry#/proc/}; pid=${pid%/comm}
            if [ "$name" = nginx ]; then
                read -r -d "" command < "/proc/$pid/cmdline" || true
                [[ "$command" = "nginx: master process"* ]] || continue
            fi
            kill -TERM "$pid"
            exit
        fi
    done
    exit 1' bash "$child"
    for _ in {1..20}; do
        [ "$(docker inspect -f '{{.State.Running}}' "$web")" = false ] && break
        sleep 1
    done
    [ "$(docker inspect -f '{{.State.Running}}' "$web")" = false ]
    [ "$(docker inspect -f '{{.State.ExitCode}}' "$web")" != 0 ]
    docker rm "$web" >/dev/null
done
printf 'All production image smoke checks passed.\n'
