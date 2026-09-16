import json

import httpx
import pytest

from exposed.cli import main
from exposed.declaration_api import INTERESTS_BASE_URL
from tests.declaration_fakes import DeclarationsFixture, declaration, money
from tests.fakes import TERM_START, ParliamentFixture
from tests.test_importer import run as import_members

pytestmark = pytest.mark.integration


@pytest.mark.parametrize("failure, exit_code", [(None, 0), (503, 1), ("interrupt", 130)])
def test_declaration_command_reports_normal_results(
    database_url, monkeypatch, capsys, caplog, failure, exit_code
):
    import_members(database_url, ParliamentFixture(1))
    fixture = DeclarationsFixture(declaration(fields=[money("janky amount")]), declaration(102))

    def handle(request):
        assert str(request.url).startswith(INTERESTS_BASE_URL)
        if failure == "interrupt":
            raise KeyboardInterrupt
        if isinstance(failure, int):
            return httpx.Response(failure)
        return fixture.handle(request)

    # Replace only the outbound HTTP transport; CLI, client, importer and DB are real.
    original_client = httpx.Client
    monkeypatch.setattr(
        httpx, "Client", lambda **kw: original_client(**kw, transport=httpx.MockTransport(handle))
    )
    monkeypatch.setenv("DATABASE_URL", database_url)
    monkeypatch.setenv("PARLIAMENT_TERM_START", TERM_START.isoformat())

    assert main(["import-declarations"]) == exit_code
    summary = json.loads(capsys.readouterr().out)
    assert summary["status"] == ("succeeded" if exit_code == 0 else "failed")
    if exit_code == 0:
        assert "Rejected declaration 101" in caplog.text
        assert "janky amount" in caplog.text
        assert summary["declarations"] == 1
