"""Operator commands and the NDJSON API worker used by the Rust application."""

import argparse
import json
import logging
import os
import sys
from datetime import date
from pathlib import Path
from urllib.parse import urlsplit

import httpx
from dotenv import load_dotenv

from exposed.operator import OperatorError, run_import
from exposed.source import SourceClient, SourceError


def source_worker(source: SourceClient) -> int:
    for line in sys.stdin:
        try:
            response = source.fetch(json.loads(line))
        except (ValueError, KeyError, TypeError, SourceError) as exc:
            print(json.dumps({"error": str(exc)}), flush=True)
            return 1
        print(json.dumps(response), flush=True)
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Fetch Parliament evidence for the Rust application"
    )
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("source", help="Serve raw API requests over NDJSON stdin/stdout")
    for name in ("import-members", "import-declarations", "initialize"):
        command = commands.add_parser(name)
        command.add_argument(
            "--term-start", help="Configured Parliament election date (YYYY-MM-DD)"
        )
        command.add_argument(
            "--app-url", help="Rust operator API URL (default http://127.0.0.1:7000)"
        )
    args = parser.parse_args(argv)
    logging.basicConfig(level=logging.INFO, format="%(levelname)s %(message)s")
    logging.getLogger("httpx").setLevel(logging.WARNING)
    with httpx.Client(
        timeout=httpx.Timeout(30, connect=10),
        headers={"Accept": "application/json", "User-Agent": "exposed-api-client/0.1"},
    ) as client:
        source = SourceClient(client)
        if args.command == "source":
            return source_worker(source)
        # Explicit working-directory configuration only. This package never reads DATABASE_URL.
        load_dotenv(Path.cwd() / ".env", override=False)
        term = args.term_start or os.environ.get("PARLIAMENT_TERM_START")
        if term is not None:
            try:
                if date.fromisoformat(term).isoformat() != term:
                    raise ValueError
            except ValueError:
                parser.error("--term-start/PARLIAMENT_TERM_START must be YYYY-MM-DD")
        app_url = args.app_url or os.environ.get("EXPOSED_APP_URL", "http://127.0.0.1:7000")
        parsed_url = urlsplit(app_url)
        if parsed_url.scheme not in {"http", "https"} or not parsed_url.netloc:
            parser.error("--app-url/EXPOSED_APP_URL must be an HTTP or HTTPS URL")
        headers = {}
        if token := os.environ.get("EXPOSED_IMPORT_TOKEN"):
            headers["Authorization"] = f"Bearer {token}"
        with httpx.Client(base_url=app_url, headers=headers, timeout=330) as app:
            try:
                kind = {
                    "import-members": "members",
                    "import-declarations": "declarations",
                    "initialize": "initialize",
                }[args.command]
                result = run_import(kind, term, app, source)
            except (SourceError, OperatorError) as exc:
                print(json.dumps({"status": "failed", "error": str(exc)}))
                return 1
            except KeyboardInterrupt:
                print(json.dumps({"status": "failed", "error": "Interrupted"}))
                return 130
        print(json.dumps(result, sort_keys=True))
        return 0
