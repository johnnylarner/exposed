import os
import subprocess
from pathlib import Path
from urllib.parse import urlsplit, urlunsplit
from uuid import uuid4

import pytest
from psycopg import sql

from exposed.importer import connect

ROOT = Path(__file__).resolve().parents[2]


@pytest.fixture
def empty_database_url():
    admin_url = os.environ.get("EXPOSED_TEST_ADMIN_DSN")
    if not admin_url:
        pytest.skip(
            "Set EXPOSED_TEST_ADMIN_DSN to a local database with CREATE DATABASE permission"
        )
    parts = urlsplit(admin_url)
    if parts.scheme not in {"postgres", "postgresql"}:
        pytest.fail("EXPOSED_TEST_ADMIN_DSN must be a PostgreSQL URL")
    # Only this newly created, randomly named database is modified or removed by tests.
    database = "exposed_test_" + uuid4().hex
    with connect(admin_url) as admin:
        admin.execute(sql.SQL("CREATE DATABASE {}").format(sql.Identifier(database)))
    url = urlunsplit(parts._replace(path="/" + database))
    try:
        yield url
    finally:
        with connect(admin_url) as admin:
            admin.execute(sql.SQL("DROP DATABASE {} WITH (FORCE)").format(sql.Identifier(database)))

