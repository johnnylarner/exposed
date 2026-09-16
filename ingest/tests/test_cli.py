import json
from datetime import date

import httpx
import pytest

from exposed.cli import main, run_cli
from exposed.core.errors import ImportFailed, StorageError
from tests.fakes import ParliamentFixture


def test_cli_passes_explicit_configuration_to_the_refresh_and_prints_its_summary(
    monkeypatch, tmp_path, capsys
):
    monkeypatch.chdir(tmp_path)
    monkeypatch.setenv("DATABASE_URL", "postgresql://configured-database")
    monkeypatch.setenv("PARLIAMENT_TERM_START", "2020-01-01")
    (tmp_path / ".env").write_text("DATABASE_URL=postgresql://file-database\n")

    def run(database_url: str, term_start: date) -> dict[str, object]:
        assert database_url == "postgresql://configured-database"
        assert term_start == date(2024, 7, 4)
        return {"status": "succeeded", "members": 2}

    status = run_cli(["import-members", "--term-start", "2024-07-04"], run)

    assert status == 0
    assert json.loads(capsys.readouterr().out) == {"status": "succeeded", "members": 2}


@pytest.mark.parametrize(
    ("failure", "status", "message"),
    [
        (ImportFailed("Source unavailable"), 1, "Source unavailable"),
        (ImportFailed("Import interrupted", interrupted=True), 130, "Import interrupted"),
        (KeyboardInterrupt(), 130, "Interrupted"),
        (
            StorageError("Database operation failed (OperationalError, SQLSTATE unknown)"),
            1,
            "Database operation failed (OperationalError, SQLSTATE unknown)",
        ),
    ],
)
def test_cli_preserves_failure_json_and_exit_codes(
    failure, status, message, monkeypatch, tmp_path, capsys
):
    monkeypatch.chdir(tmp_path)
    monkeypatch.setenv("DATABASE_URL", "postgresql://configured-database")

    def run(database_url: str, term_start: date) -> dict[str, object]:
        raise failure

    assert run_cli(["import-members", "--term-start", "2024-07-04"], run) == status
    assert json.loads(capsys.readouterr().out) == {"status": "failed", "error": message}


@pytest.mark.parametrize(
    ("database_url", "term_start"),
    [
        (None, "2024-07-04"),
        ("postgresql://configured", None),
        ("postgresql://configured", "20240704"),
        ("postgresql://configured", "2024-02-30"),
    ],
)
def test_invalid_cli_configuration_exits_before_starting_a_refresh(
    database_url, term_start, monkeypatch, tmp_path, capsys
):
    monkeypatch.chdir(tmp_path)
    for key, value in {"DATABASE_URL": database_url, "PARLIAMENT_TERM_START": term_start}.items():
        if value is None:
            monkeypatch.delenv(key, raising=False)
        else:
            monkeypatch.setenv(key, value)

    def run(database_url: str, term_start: date) -> dict[str, object]:
        pytest.fail("Invalid CLI configuration must not start a refresh")

    with pytest.raises(SystemExit) as error:
        run_cli(["import-members"], run)

    assert error.value.code == 2
    assert capsys.readouterr().out == ""


@pytest.mark.integration
def test_production_cli_wiring_refreshes_members_in_postgresql(
    database_url, monkeypatch, tmp_path, capsys
):
    fixture = ParliamentFixture(2)
    fixture.leave(2, lords=True)
    client_type = httpx.Client

    def local_client(*args, **kwargs):
        kwargs["transport"] = httpx.MockTransport(fixture.handle)
        return client_type(*args, **kwargs)

    # Substitute only the external HTTP transport; run the real CLI, composition,
    # source adapter, refresh and PostgreSQL adapter against a temporary database.
    monkeypatch.setattr(httpx, "Client", local_client)
    monkeypatch.chdir(tmp_path)
    monkeypatch.setenv("DATABASE_URL", database_url)
    monkeypatch.setenv("PARLIAMENT_TERM_START", "2024-07-04")

    assert main(["import-members"]) == 0
    result = json.loads(capsys.readouterr().out)
    assert (
        result["status"],
        result["inserted"],
        result["current_commons"],
        result["former_commons"],
    ) == ("succeeded", 2, 1, 1)
