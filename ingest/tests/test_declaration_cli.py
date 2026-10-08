import json

import pytest

from exposed.cli import run_cli
from tests.fakes import TERM_START


@pytest.mark.parametrize("interrupted", [False, True])
def test_declaration_cli_uses_injected_command_and_maps_failures(
    interrupted,
    monkeypatch,
    tmp_path,
    capsys,
):
    from exposed.core.errors import ImportFailed

    monkeypatch.chdir(tmp_path)
    monkeypatch.setenv("DATABASE_URL", "postgresql://configured")
    monkeypatch.setenv("PARLIAMENT_TERM_START", TERM_START.isoformat())

    def members(url, term):
        pytest.fail("Declaration command must not call the member use case")

    def declarations(url, term):
        assert url == "postgresql://configured"
        assert term == TERM_START
        raise ImportFailed("Declaration refresh failed", interrupted=interrupted)

    assert run_cli(["import-declarations"], members, run_declarations=declarations) == (
        130 if interrupted else 1
    )
    assert json.loads(capsys.readouterr().out) == {
        "status": "failed",
        "error": "Declaration refresh failed",
    }
