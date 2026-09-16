"""Local and scheduler-friendly command line interface."""

import argparse
import json
import logging
import os
from collections.abc import Callable
from datetime import date
from pathlib import Path

from dotenv import load_dotenv

from exposed.core.errors import ImportFailed, ImportValidationError, StorageError, safe_error


def run_cli(
    argv: list[str] | None,
    run: Callable[[str, date], dict[str, object]],
) -> int:
    # Only the working directory's explicit .env; never search unrelated parent directories.
    # Existing environment variables (including GitHub Actions secrets) take precedence.
    load_dotenv(Path.cwd() / ".env", override=False)
    parser = argparse.ArgumentParser(description="Import Commons members into PostgreSQL")
    sub = parser.add_subparsers(dest="command", required=True)
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
    try:
        term_start = date.fromisoformat(args.term_start)
        if term_start.isoformat() != args.term_start:
            raise ValueError
    except ValueError, TypeError:
        parser.error("Set PARLIAMENT_TERM_START or --term-start to an election date (YYYY-MM-DD)")
    logging.basicConfig(level=logging.INFO, format="%(levelname)s %(message)s")
    logging.getLogger("httpx").setLevel(logging.WARNING)
    try:
        result = run(database_url, term_start)
    except ImportFailed as exc:
        print(json.dumps({"status": "failed", "error": str(exc)}))
        return 130 if exc.interrupted else 1
    except (StorageError, ImportValidationError) as exc:
        print(json.dumps({"status": "failed", "error": safe_error(exc)}))
        logging.error(
            "Check DATABASE_URL and run 'make migrate' from the repository root before importing"
        )
        return 1
    except KeyboardInterrupt:
        print(json.dumps({"status": "failed", "error": "Interrupted"}))
        return 130
    print(json.dumps(result, sort_keys=True))
    return 0


def main(argv: list[str] | None = None) -> int:
    """Retain the installed console entry point; concrete wiring lives at startup."""
    from exposed.composition import import_members

    return run_cli(argv, import_members)
