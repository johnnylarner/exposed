#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
: "${DATABASE_URL:?Supply a PostgreSQL administrator URL for isolated SQLx test databases}"
contract_output=$(mktemp -d)
trap 'rm -rf "$contract_output"' EXIT
export EXPOSED_CONTRACT_OUTPUT_DIR="$contract_output"
SQLX_OFFLINE=false cargo test --locked -p exposed --lib outbound::entity_details::tests
npm --prefix frontend run check
npm --prefix frontend run build
(cd frontend && PLAYWRIGHT_BROWSERS_PATH=0 npx playwright test tests/member-summary.spec.ts tests/entity-details.spec.ts)
(cd frontend && PLAYWRIGHT_BROWSERS_PATH=0 npx playwright test --config playwright.contract.config.ts)
