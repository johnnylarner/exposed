"""Local and scheduler-friendly command line interface."""

import argparse
import json
import logging
import os
from datetime import date
from pathlib import Path

import httpx
import psycopg
from dotenv import load_dotenv

from exposed.api import BASE_URL, MembersAPI
from exposed.db import migrate
from exposed.importer import ImportFailed, connect, run_import, safe_error
from exposed.models import ImportValidationError


def main(argv: list[str] | None = None) -> int:
    # Only the working directory's explicit .env; never search unrelated parent directories.
    # Existing environment variables (including GitHub Actions secrets) take precedence.
    load_dotenv(Path.cwd() / ".env", override=False)
    parser = argparse.ArgumentParser(description="Import Commons members into PostgreSQL")
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("migrate", help="Apply pending database migrations")
    importer = sub.add_parser(
        "import-members", help="Refresh all Commons service in this Parliament"
    )
    importer.add_argument(
        "--term-start",
        default=os.environ.get("PARLIAMENT_TERM_START"),
        help="Election day (YYYY-MM-DD); defaults to PARLIAMENT_TERM_START",
    )
    args = parser.parse_args(argv)
    database_url = os.environ.get("DATABASE_URL")
    if not database_url:
        parser.error("Set DATABASE_URL in the environment or the working directory's .env")
    term_start = None
    if args.command == "import-members":
        try:
            term_start = date.fromisoformat(args.term_start)
            if term_start.isoformat() != args.term_start:
                raise ValueError
        except ValueError, TypeError:
            parser.error(
                "Set PARLIAMENT_TERM_START or --term-start to an election date (YYYY-MM-DD)"
            )
    logging.basicConfig(level=logging.INFO, format="%(levelname)s %(message)s")
    logging.getLogger("httpx").setLevel(logging.WARNING)
    try:
        if args.command == "migrate":
            with connect(database_url) as conn:
                result = {"status": "succeeded", "migrations_applied": migrate(conn)}
        else:
            assert term_start is not None
            with httpx.Client(
                base_url=BASE_URL,
                timeout=httpx.Timeout(30, connect=10),
                headers={"Accept": "application/json", "User-Agent": "exposed-member-importer/0.1"},
            ) as client:
                result = run_import(database_url, term_start, MembersAPI(client))
    except ImportFailed as exc:
        print(json.dumps({"status": "failed", "error": str(exc)}))
        return 130 if exc.interrupted else 1
    except (psycopg.Error, ImportValidationError) as exc:
        print(json.dumps({"status": "failed", "error": safe_error(exc)}))
        logging.error("Check DATABASE_URL and run 'exposed migrate' before importing")
        return 1
    except KeyboardInterrupt:
        print(json.dumps({"status": "failed", "error": "Interrupted"}))
        return 130
    print(json.dumps(result, sort_keys=True))
    return 0
