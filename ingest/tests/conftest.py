import os
from uuid import uuid4

import pytest
from psycopg import sql
from psycopg.conninfo import make_conninfo

from exposed.db import migrate
from exposed.importer import connect


@pytest.fixture
def database_url():
    admin_url = os.environ.get("EXPOSED_TEST_ADMIN_DSN")
    if not admin_url:
        pytest.skip(
            "Set EXPOSED_TEST_ADMIN_DSN to a local database with CREATE DATABASE permission"
        )
    # Only this newly created, randomly named database is modified or removed by tests.
    database = "exposed_test_" + uuid4().hex
    with connect(admin_url) as admin:
        admin.execute(sql.SQL("CREATE DATABASE {}").format(sql.Identifier(database)))
    url = make_conninfo(admin_url, dbname=database)
    try:
        with connect(url) as conn:
            migrate(conn)
        yield url
    finally:
        with connect(admin_url) as admin:
            admin.execute(sql.SQL("DROP DATABASE {} WITH (FORCE)").format(sql.Identifier(database)))
