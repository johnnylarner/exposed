"""Run with ingest/.venv/bin/python ingest/scripts/backfill_funders.py [--apply]."""

from exposed.backfill_funders import main

if __name__ == "__main__":
    raise SystemExit(main())
