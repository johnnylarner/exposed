#!/usr/bin/env bash
set -uo pipefail

"$@" &
server_pid=$!
node /app/frontend/build &
frontend_pid=$!

stop_children() {
    kill -TERM "$server_pid" "$frontend_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
    wait "$frontend_pid" 2>/dev/null || true
}
trap 'stop_children; exit 0' TERM INT

wait -n "$server_pid" "$frontend_pid"
status=$?
stop_children
if [ "$status" -eq 0 ]; then
    status=1
fi
exit "$status"
