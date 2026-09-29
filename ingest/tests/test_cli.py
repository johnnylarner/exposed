import httpx
import pytest

from exposed.cli import main


@pytest.mark.parametrize("command", ["import-members", "import-declarations", "initialize"])
@pytest.mark.parametrize("configuration", ["argument", "environment"])
@pytest.mark.parametrize(
    "url",
    [
        "http://127.0.0.1:not-a-port",
        "http://[::1",
        "http://127.0.0.1:65536",
        "http://",
        "http://\x01example.com",
    ],
)
def test_malformed_operator_url_is_a_configuration_error(
    command, configuration, url, monkeypatch, capsys, tmp_path
):
    monkeypatch.chdir(tmp_path)
    monkeypatch.delenv("PARLIAMENT_TERM_START", raising=False)
    monkeypatch.delenv("EXPOSED_APP_URL", raising=False)

    def unexpected_request(*args, **kwargs):
        pytest.fail("Invalid configuration must not send an HTTP request")

    monkeypatch.setattr(httpx.Client, "send", unexpected_request)
    arguments = [command]
    if configuration == "argument":
        arguments.extend(["--app-url", url])
    else:
        monkeypatch.setenv("EXPOSED_APP_URL", url)

    with pytest.raises(SystemExit) as error:
        main(arguments)

    assert error.value.code == 2
    output = capsys.readouterr()
    assert output.out == ""
    assert "--app-url/EXPOSED_APP_URL" in output.err
    assert "URL" in output.err
    assert "Traceback" not in output.err
