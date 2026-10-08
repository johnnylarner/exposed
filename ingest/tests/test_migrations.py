import os
import subprocess
from pathlib import Path

import pytest

from exposed.importer import connect

pytestmark = pytest.mark.integration
ROOT = Path(__file__).resolve().parents[2]


def test_single_baseline_reverts_and_reapplies(empty_database_url):
    args = [
        os.environ.get("SQLX", "sqlx"),
        "migrate",
    ]
    options = [
        "--config",
        str(ROOT / "sqlx.toml"),
        "--source",
        str(ROOT / "db/migrations"),
        "--database-url",
        empty_database_url,
    ]
    for action in ("run", "revert", "run"):
        subprocess.run([*args, action, *options], check=True, capture_output=True, text=True)
        with connect(empty_database_url) as conn:
            result = conn.execute("SELECT to_regclass('exposed.funders') AS funders").fetchone()
            assert result is not None
            assert (result["funders"] is None) == (action == "revert")
    with connect(empty_database_url) as conn:
        assert conn.execute("SELECT version FROM public._sqlx_migrations").fetchall() == [
            {"version": 20260915000000}
        ]
