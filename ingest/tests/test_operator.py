import httpx
import pytest

from exposed.operator import OperatorError, run_import
from exposed.source import SourceClient


def test_committed_acknowledgement_controls_next_fetch_and_result():
    events = []

    def parliament(request):
        events.append("fetch")
        return httpx.Response(200, json={"raw": "untouched"})

    def application(request):
        if request.method == "DELETE":
            return httpx.Response(200, json={"status": "cancelled"})
        if request.url.path.endswith("evidence"):
            events.append("commit")
            return httpx.Response(200, json={"result": {"status": "succeeded", "members": 1}})
        events.append("begin")
        return httpx.Response(
            200, json={"sequence": 0, "request": {"operation": "current-members", "offset": 0}}
        )

    with httpx.Client(transport=httpx.MockTransport(parliament)) as api:
        with httpx.Client(base_url="http://app", transport=httpx.MockTransport(application)) as app:
            result = run_import("members", None, app, SourceClient(api, sleep=lambda _: None))
    assert events == ["begin", "fetch", "commit"]
    assert result == {"status": "succeeded", "members": 1}


def test_server_failure_stops_acquisition():
    def application(request):
        return httpx.Response(200, json={"error": "Another import is running"})

    def unexpected(request):
        pytest.fail("No Parliament request should be made")

    with httpx.Client(transport=httpx.MockTransport(unexpected)) as api:
        with httpx.Client(base_url="http://app", transport=httpx.MockTransport(application)) as app:
            with pytest.raises(OperatorError, match="Another import"):
                run_import("members", None, app, SourceClient(api))
