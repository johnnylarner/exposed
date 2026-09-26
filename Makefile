.DEFAULT_GOAL := help
PYTHON ?= python3
SQLX ?= sqlx
VENV_PYTHON := .venv/bin/python
SQLX_CMD = cd ingest && $(SQLX)
SQLX_MIGRATION_ARGS = --config ../sqlx.toml --source ../db/migrations
EXPOSED_TEST_ADMIN_DSN ?= postgresql://exposed:exposed_local_dev@localhost:55432/postgres?sslmode=disable
export EXPOSED_TEST_ADMIN_DSN
export SQLX

.PHONY: db-start db-stop db-nuke db-migrate db-revert db-migration-status db-add-migration import-members import-declarations verify lint format typecheck test test-unit check frontend-dev frontend-check

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

import-members: ## Refresh member data using ingest/.env or environment variables
	cd ingest && $(VENV_PYTHON) -m exposed import-members

import-declarations: ## Refresh declarations for the stored member cohort
	cd ingest && $(VENV_PYTHON) -m exposed import-declarations

verify: ## Check domain data in the local Compose database
	docker compose exec -T postgres psql -U exposed -d exposed < ingest/scripts/verify.sql

lint: ## Check Python lint and formatting
	cd ingest && $(VENV_PYTHON) -m ruff check src tests scripts
	cd ingest && $(VENV_PYTHON) -m ruff format --check src tests scripts

format: ## Format Python code
	cd ingest && $(VENV_PYTHON) -m ruff format src tests scripts

typecheck: ## Run Pyright
	cd ingest && $(VENV_PYTHON) -m pyright

test: ## Run all tests, including isolated PostgreSQL databases (requires db-start)
	cd ingest && $(VENV_PYTHON) -m pytest -q

test-unit: ## Run tests without PostgreSQL
	cd ingest && $(VENV_PYTHON) -m pytest -q -m 'not integration'

check: lint typecheck test ## Run importer lint, type checking and tests

frontend-dev: ## Start the Svelte frontend and API with live reload
	docker compose up --build frontend

frontend-check: ## Check and build the frontend, then run its browser tests
	npm --prefix frontend run format:check
	npm --prefix frontend run check
	npm --prefix frontend run build
	npm --prefix frontend test
