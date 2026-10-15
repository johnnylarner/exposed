#!/usr/bin/env bash
set -uo pipefail

exposed server &
server_pid=$!
nginx -g 'daemon off;' &
nginx_pid=$!

stop_children() {
    kill -TERM "$server_pid" "$nginx_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
    wait "$nginx_pid" 2>/dev/null || true
}
trap 'stop_children; exit 0' TERM INT

wait -n "$server_pid" "$nginx_pid"
status=$?
stop_children
if [ "$status" -eq 0 ]; then
    status=1
fi
exit "$status"
