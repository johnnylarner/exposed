.DEFAULT_GOAL := help
PYTHON ?= python3
SQLX ?= sqlx
VENV_PYTHON := .venv/bin/python
SQLX_CMD = cd ingest && $(SQLX)
SQLX_MIGRATION_ARGS = --config ../sqlx.toml --source ../db/migrations
EXPOSED_TEST_ADMIN_DSN ?= postgresql://exposed:exposed_local_dev@localhost:55432/postgres?sslmode=disable
export EXPOSED_TEST_ADMIN_DSN
export SQLX
RUST_DATABASE_URL ?= postgresql://exposed:exposed_local_dev@localhost:55432/exposed?options=-csearch_path%3Dexposed,public

.PHONY: db-start db-stop db-nuke db-migrate db-revert db-migration-status db-add-migration import-members import-declarations initialize verify lint format typecheck test test-unit rust-check test-rust check

db-start: ## Start the local PostgreSQL database
	docker compose up -d --wait postgres

db-stop: ## Stop PostgreSQL, keeping its data
	docker compose stop postgres

db-nuke: ## Delete the configured database and recreate it from the baseline
	$(SQLX_CMD) database reset -yf $(SQLX_MIGRATION_ARGS)


db-migrate: ## Apply the development schema baseline using SQLx
	$(SQLX_CMD) migrate run $(SQLX_MIGRATION_ARGS)

db-revert: ## Revert the baseline, deleting all domain tables and their data
	$(SQLX_CMD) migrate revert $(SQLX_MIGRATION_ARGS)

db-migration-status: ## Show SQLx migration status
	$(SQLX_CMD) migrate info $(SQLX_MIGRATION_ARGS)

db-add-migration:
	@echo "Edit db/migrations/20260915000000_initial.{up,down}.sql; development uses one baseline version (see db/README.md)." >&2
	@exit 1

import-members: ## Manually refresh members through the running Rust application
	cd ingest && $(VENV_PYTHON) -m exposed import-members

import-declarations: ## Manually request a declaration refresh (also scheduled by Rust)
	cd ingest && $(VENV_PYTHON) -m exposed import-declarations

initialize: ## Initialize members, then declarations; safe to repeat
	cd ingest && $(VENV_PYTHON) -m exposed initialize

verify: ## Check domain data in the local Compose database
	docker compose exec -T postgres psql -U exposed -d exposed < ingest/scripts/verify.sql

lint: ## Check Python lint and formatting
	cd ingest && $(VENV_PYTHON) -m ruff check src tests scripts
	cd ingest && $(VENV_PYTHON) -m ruff format --check src tests scripts

format: ## Format Python code
	cd ingest && $(VENV_PYTHON) -m ruff format src tests scripts

typecheck: ## Run Pyright
	cd ingest && $(VENV_PYTHON) -m pyright

test: ## Run Python API and operator tests (no PostgreSQL required)
	cd ingest && $(VENV_PYTHON) -m pytest -q

test-unit: ## Run tests without PostgreSQL
	cd ingest && $(VENV_PYTHON) -m pytest -q -m 'not integration'

rust-check: ## Check Rust formatting and all build targets
	cargo fmt --all -- --check
	DATABASE_URL='$(RUST_DATABASE_URL)' cargo check --all-targets

test-rust: ## Run Rust tests with isolated PostgreSQL databases
	DATABASE_URL='$(RUST_DATABASE_URL)' cargo test --workspace

check: lint typecheck rust-check test test-rust ## Run lint, type checking and all tests
