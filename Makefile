.DEFAULT_GOAL := help
PYTHON ?= python3
DBMATE ?= dbmate
VENV_PYTHON := .venv/bin/python
DBMATE_CMD = $(DBMATE) --env-file ingest/.env --migrations-dir db/migrations --migrations-table public.schema_migrations --no-dump-schema
EXPOSED_TEST_ADMIN_DSN ?= postgresql://exposed:exposed_local_dev@localhost:55432/postgres?sslmode=disable
export EXPOSED_TEST_ADMIN_DSN
export DBMATE
export name

.PHONY: db-start db-stop db-nuke db-migrate db-add-migration import-members verify lint format typecheck test test-unit check

db-start: ## Start the local PostgreSQL database
	docker compose up -d --wait

db-stop: ## Stop PostgreSQL, keeping its data
	docker compose stop

db-nuke:
	$(DBMATE_CMD) drop exposed
	$(DBMATE_CMD) create exposed
	$(DBMATE_CMD) migrate


db-migrate:
	$(DBMATE_CMD) migrate

db-add-migration:
	$(DBMATE_CMD) new "$$name"

import-members: ## Refresh member data using ingest/.env or environment variables
	cd ingest && $(VENV_PYTHON) -m exposed import-members

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

check: lint typecheck test ## Run lint, type checking and the full test suite
