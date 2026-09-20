"""Maintenance command: enrich existing funding rows from the current source evidence."""

import argparse
import json
import logging
import os
from dataclasses import asdict, dataclass
from datetime import UTC, datetime
from pathlib import Path

import httpx
import psycopg
from dotenv import load_dotenv

from exposed.adapters.declarations import INTERESTS_BASE_URL, DeclarationsAPI
from exposed.adapters.postgres import DatabaseConnection, connect, storage_error
from exposed.core.declarations import FundingEntry, RetrievedDeclaration
from exposed.core.errors import DeclarationParseError, ImportValidationError, safe_error
from exposed.core.funding_backfill import plan_funding_backfill
from exposed.core.refresh_declarations import DeclarationSources

logger = logging.getLogger(__name__)


@dataclass
class BackfillCounts:
    declarations: int = 0
    changed_entries: int = 0
    unchanged_entries: int = 0
    skipped_declarations: int = 0
    entries_without_status: int = 0
    companies_without_number: int = 0


def backfill_funders(
    conn: DatabaseConnection,
    api: DeclarationsAPI,
    *,
    apply: bool = False,
    declaration_ids: list[int] | None = None,
    batch_size: int = 100,
) -> dict[str, object]:
    """Preview or fill missing fields; each declaration commits independently.

    Only existing funded declarations are queried. The database must already
    have the funder-identification columns from the current baseline. As with imports,
    run without concurrent imports/migrations.
    """
    if not 1 <= batch_size <= 100:
        raise ValueError("batch_size must be between 1 and 100")
    targets = conn.execute(
        """SELECT d.source_declaration_id, m.parliament_member_id
           FROM exposed.declarations d
           JOIN exposed.members m ON m.id = d.member_id
           WHERE EXISTS (SELECT 1 FROM exposed.funding_entries f
                         WHERE f.source_declaration_id = d.source_declaration_id)
             AND (%s::integer[] IS NULL OR d.source_declaration_id = ANY(%s))
           ORDER BY d.source_declaration_id""",
        (declaration_ids, declaration_ids),
    ).fetchall()
    if declaration_ids is not None:
        found = {row["source_declaration_id"] for row in targets}
        if missing := set(declaration_ids) - found:
            raise ImportValidationError(f"No stored funding for declaration IDs {sorted(missing)}")
    counts = BackfillCounts(declarations=len(targets))
    for start in range(0, len(targets), batch_size):
        batch = targets[start : start + batch_size]
        ids = {row["source_declaration_id"] for row in batch}
        sources = DeclarationSources(api)
        for page in api.declarations_by_ids(sorted(ids)):
            for payload in page.items:
                record = RetrievedDeclaration(
                    identifier=payload.get("id") if isinstance(payload, dict) else None,
                    payload=payload,
                    fetched_at=datetime.now(UTC),
                    context="funder backfill",
                )
                if record.source_id not in ids:
                    raise ImportValidationError("API returned an unrequested declaration ID")
                sources.remember(record)
        for target in batch:
            source_id = target["source_declaration_id"]
            if source_id not in sources.records:
                logger.warning("Skipped declaration %s: API did not return it", source_id)
                counts.skipped_declarations += 1
                continue
            try:
                declaration = sources.resolve(
                    sources.records[source_id], target["parliament_member_id"]
                )
                with conn.transaction():
                    # Serialize against the normal writer before reading its funding rows.
                    current = conn.execute(
                        """SELECT m.parliament_member_id FROM exposed.declarations d
                           JOIN exposed.members m ON m.id = d.member_id
                           WHERE d.source_declaration_id = %s FOR UPDATE OF d""",
                        (source_id,),
                    ).fetchone()
                    if current is None or current != {
                        "parliament_member_id": declaration.member_source_id
                    }:
                        raise DeclarationParseError("Stored declaration identity changed")
                    rows = conn.execute(
                        """SELECT id, funder, amount, currency, payment_type,
                                  donor_status, company_number
                           FROM exposed.funding_entries WHERE source_declaration_id = %s
                           ORDER BY id FOR UPDATE""",
                        (source_id,),
                    ).fetchall()
                    stored = tuple(FundingEntry.model_validate(row) for row in rows)
                    enriched = plan_funding_backfill(stored, declaration.funding)
                    updates = [
                        (new.donor_status, new.company_number, row["id"])
                        for row, old, new in zip(rows, stored, enriched, strict=True)
                        if old != new
                    ]
                    if apply and updates:
                        with conn.cursor() as cursor:
                            cursor.executemany(
                                """UPDATE exposed.funding_entries
                                   SET donor_status = %s, company_number = %s WHERE id = %s""",
                                updates,
                            )
                counts.changed_entries += len(updates)
                counts.unchanged_entries += len(rows) - len(updates)
                counts.entries_without_status += sum(e.donor_status is None for e in enriched)
                counts.companies_without_number += sum(
                    e.donor_status == "Company" and e.company_number is None for e in enriched
                )
                logger.info(
                    "%s declaration %s: %s funding entries",
                    "Updated" if apply else "Would update",
                    source_id,
                    len(updates),
                )
            except DeclarationParseError as exc:
                logger.warning("Skipped declaration %s: %s", source_id, exc)
                counts.skipped_declarations += 1
    return {
        "status": "partial" if counts.skipped_declarations else "succeeded",
        "mode": "apply" if apply else "dry-run",
        **asdict(counts),
    }


def positive_id(value: str) -> int:
    parsed = int(value)
    if parsed <= 0:
        raise argparse.ArgumentTypeError("declaration ID must be positive")
    return parsed


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--apply", action="store_true", help="write changes; default is a dry run")
    parser.add_argument("--declaration-id", type=positive_id, action="append")
    parser.add_argument(
        "--env-file", type=Path, default=Path(__file__).resolve().parents[2] / ".env"
    )
    args = parser.parse_args(argv)
    load_dotenv(args.env_file, override=False)
    database_url = os.environ.get("DATABASE_URL")
    if not database_url:
        parser.error("DATABASE_URL is required (environment or ingest/.env)")
    logging.basicConfig(level=logging.INFO, format="%(levelname)s %(message)s")
    try:
        with (
            connect(database_url) as conn,
            httpx.Client(
                base_url=INTERESTS_BASE_URL,
                timeout=httpx.Timeout(30, connect=10),
                headers={"Accept": "application/json", "User-Agent": "exposed-funder-backfill/0.1"},
            ) as client,
        ):
            result = backfill_funders(
                conn, DeclarationsAPI(client), apply=args.apply, declaration_ids=args.declaration_id
            )
        print(json.dumps(result))
        return 0 if result["status"] == "succeeded" else 1
    except KeyboardInterrupt:
        logger.error("Backfill interrupted; completed declarations remain committed")
        return 130
    except Exception as exc:
        message = str(storage_error(exc)) if isinstance(exc, psycopg.Error) else safe_error(exc)
        logger.error("Backfill failed: %s; completed declarations remain committed", message)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
