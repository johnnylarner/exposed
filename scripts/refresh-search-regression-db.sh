#!/usr/bin/env bash
set -euo pipefail

if [[ $# -gt 1 || ( $# -eq 1 && "$1" != "--help" ) ]]; then
  echo "Usage: $0 [--help]" >&2
  exit 2
fi

if [[ "${1:-}" == "--help" ]]; then
  cat <<'HELP'
Usage: refresh-search-regression-db.sh [--help]

Replace exposed_search_regression with a fresh clone of exposed.
Existing target connections are terminated. exposed_search_regression_stage
is reserved disposable staging storage and is deleted on each attempt.

Stop the app and disconnect other clients from exposed before running.
If cloning fails because exposed is busy, the previous target is preserved.
The script does not stop the app or disconnect clients from exposed.
Uses the repository's Compose file and honors COMPOSE_PROJECT_NAME.
HELP
  exit 0
fi

command -v docker >/dev/null || { echo "docker is required" >&2; exit 1; }
repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)

echo "Refreshing exposed_search_regression from exposed."
echo "Stop the app and disconnect clients from exposed before running."
if ! docker compose --file "$repo_root/compose.yaml" exec -T postgres \
  psql -X -v ON_ERROR_STOP=1 -U exposed -d postgres <<'SQL'
DO $$
BEGIN
  IF NOT pg_try_advisory_lock(hashtextextended('exposed:refresh-search-regression-db', 0)) THEN
    RAISE EXCEPTION 'Another search regression database refresh is running. Retry after it finishes.';
  END IF;
END;
$$;
DROP DATABASE IF EXISTS exposed_search_regression_stage WITH (FORCE);
CREATE DATABASE exposed_search_regression_stage TEMPLATE exposed ALLOW_CONNECTIONS false;
DROP DATABASE IF EXISTS exposed_search_regression WITH (FORCE);
ALTER DATABASE exposed_search_regression_stage RENAME TO exposed_search_regression;
ALTER DATABASE exposed_search_regression ALLOW_CONNECTIONS true;
SQL
then
  echo "Refresh failed. See the PostgreSQL or Docker error above." >&2
  echo "If exposed is busy, stop the app and disconnect its other clients before retrying." >&2
  exit 1
fi

echo "Refreshed exposed_search_regression."
echo "For future refreshes, stop the app and disconnect clients from exposed first."
