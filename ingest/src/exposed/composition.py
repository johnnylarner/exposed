"""Construct production adapters and own their lifetimes and observation clock."""

from datetime import date, datetime
from zoneinfo import ZoneInfo

import httpx
import psycopg

from exposed.adapters.declaration_postgres import PostgresDeclarationStore
from exposed.adapters.declarations import INTERESTS_BASE_URL, DeclarationsAPI
from exposed.adapters.parliament import BASE_URL, MembersAPI
from exposed.adapters.postgres import PostgresStore, connect, storage_error
from exposed.core.errors import ImportFailed
from exposed.core.refresh import refresh_members
from exposed.core.refresh_declarations import refresh_declarations


def run_import(
    database_url: str,
    term_start: date,
    api: MembersAPI,
    *,
    as_of: date | None = None,
) -> dict[str, object]:
    """Compatibility entry point for callers supplying a Parliament client."""
    as_of = as_of or datetime.now(ZoneInfo("Europe/London")).date()
    # Keep connection-opening failures outside ImportFailed for existing callers.
    with connect(database_url) as conn:
        return refresh_members(term_start, as_of, api, PostgresStore(conn))


def import_members(database_url: str, term_start: date) -> dict[str, object]:
    """Production CLI runner; translate connection failures at the edge."""
    try:
        with httpx.Client(
            base_url=BASE_URL,
            timeout=httpx.Timeout(30, connect=10),
            headers={"Accept": "application/json", "User-Agent": "exposed-member-importer/0.1"},
        ) as client:
            return run_import(database_url, term_start, MembersAPI(client))
    except psycopg.Error as exc:
        raise storage_error(exc) from exc


def import_declarations(database_url: str, term_start: date) -> dict[str, object]:
    """Production declaration command; own HTTP client construction at the edge."""
    with httpx.Client(
        base_url=INTERESTS_BASE_URL,
        timeout=httpx.Timeout(30, connect=10),
        headers={"Accept": "application/json", "User-Agent": "exposed-importer/0.1"},
    ) as client:
        return run_declaration_import(database_url, term_start, DeclarationsAPI(client))


def run_declaration_import(
    database_url: str,
    term_start: date,
    api: DeclarationsAPI,
) -> dict[str, object]:
    """Legacy importer signature, wiring the source and the atomic PostgreSQL store."""
    try:
        with connect(database_url) as conn:
            return refresh_declarations(term_start, api, PostgresDeclarationStore(conn))
    except psycopg.Error as exc:
        raise ImportFailed(str(storage_error(exc))) from exc
    except KeyboardInterrupt as exc:
        raise ImportFailed("Import interrupted", interrupted=True) from exc
