import httpx

from exposed.source import SourceClient


def test_api_worker_preserves_raw_evidence_without_domain_decoding():
    payload = {"items": [{"id": True, "future": {"odd": [None, 1]}}], "totalResults": 1, "skip": 0}
    requests = []

    def serve(request):
        requests.append(request)
        return httpx.Response(200, json=payload)

    with httpx.Client(transport=httpx.MockTransport(serve)) as client:
        source = SourceClient(client, sleep=lambda _: None)
        response = source.fetch({"operation": "declarations", "member": 7, "offset": 0})
    assert response["payload"] == payload
    assert requests[0].url.params["ExcludeExpired"] == "false"
    assert requests[0].url.params["ExpandChildInterests"] == "false"
    assert response["fetched_at"].endswith("+00:00")


def test_bounded_http_retries_honor_retry_after():
    replies = iter(
        [httpx.Response(429, headers={"Retry-After": "3"}), httpx.Response(200, json=[])]
    )
    sleeps = []
    with httpx.Client(transport=httpx.MockTransport(lambda _: next(replies))) as client:
        result = SourceClient(client, sleep=sleeps.append).fetch(
            {"operation": "member-histories", "ids": [2, 1]}
        )
    assert result["payload"] == []
    assert 3 in sleeps


def test_permanent_failure_is_not_retried():
    import pytest

    from exposed.source import SourceError

    requests = []

    def serve(request):
        requests.append(request)
        return httpx.Response(400)

    with httpx.Client(transport=httpx.MockTransport(serve)) as client:
        with pytest.raises(SourceError, match="400"):
            SourceClient(client, sleep=lambda _: None).fetch(
                {"operation": "current-members", "offset": 0}
            )
    assert len(requests) == 1


def test_server_failure_stops_after_four_attempts():
    import pytest

    from exposed.source import SourceError

    requests = []

    def serve(request):
        requests.append(request)
        return httpx.Response(503)

    with httpx.Client(transport=httpx.MockTransport(serve)) as client:
        with pytest.raises(SourceError, match="503"):
            SourceClient(client, sleep=lambda _: None).fetch(
                {"operation": "current-members", "offset": 0}
            )
    assert len(requests) == 4
