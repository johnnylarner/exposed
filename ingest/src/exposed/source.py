"""Parliament HTTP transport. Responses remain unmodified source evidence."""

import logging
import time
from collections.abc import Callable
from datetime import UTC, datetime
from email.utils import parsedate_to_datetime
from typing import Any

import httpx

MEMBERS = "https://members-api.parliament.uk"
INTERESTS = "https://interests-api.parliament.uk"


class SourceError(Exception):
    """A bounded API request could not complete."""


class SourceClient:
    def __init__(self, client: httpx.Client, *, sleep: Callable[[float], None] = time.sleep):
        self.client = client
        self.sleep = sleep

    def _get(self, url: str, params: httpx.QueryParams) -> Any:
        for attempt in range(4):
            self.sleep(0.2)
            response = None
            try:
                response = self.client.get(url, params=params)
            except httpx.TransportError:
                if attempt == 3:
                    raise SourceError("Parliament API transport failure") from None
            else:
                if response.status_code == 200:
                    try:
                        return response.json()
                    except ValueError:
                        raise SourceError("Parliament API returned invalid JSON") from None
                if response.status_code != 429 and response.status_code < 500 or attempt == 3:
                    raise SourceError(f"Parliament API returned HTTP {response.status_code}")
            delay = 2**attempt
            if response is not None and (retry := response.headers.get("Retry-After")):
                try:
                    delay = float(retry)
                except ValueError:
                    try:
                        delay = (parsedate_to_datetime(retry) - datetime.now(UTC)).total_seconds()
                    except ValueError, TypeError, OverflowError:
                        pass
            logging.warning("Retrying Parliament request (attempt %s/4)", attempt + 2)
            self.sleep(min(30, max(0, delay)))
        raise SourceError("Parliament API retries exhausted")

    def fetch(self, request: dict[str, Any]) -> dict[str, Any]:
        operation = request["operation"]
        offset = request.get("offset", 0)
        if operation in {"current-members", "member-candidates"}:
            params = httpx.QueryParams({"skip": offset, "take": 100})
            if operation == "current-members":
                params = params.set("House", 1).set("IsCurrentMember", "true")
            else:
                params = params.set(
                    "MembershipInDateRange.WasMemberOnOrAfter", request["term_start"]
                )
                params = params.set("MembershipInDateRange.WasMemberOnOrBefore", request["as_of"])
                params = params.set("MembershipInDateRange.WasMemberOfHouse", 1)
            url = MEMBERS + "/api/Members/Search"
        elif operation == "member-histories":
            ids = request["ids"]
            if not 1 <= len(ids) <= 100:
                raise SourceError("History requests require 1 to 100 IDs")
            params = httpx.QueryParams([("ids", str(value)) for value in sorted(ids)])
            url = MEMBERS + "/api/Members/History"
        elif operation in {"declarations", "parent"}:
            params = httpx.QueryParams(
                {
                    "Type": "Commons",
                    "ExcludeExpired": "false",
                    "ExpandChildInterests": "false",
                    "Skip": offset,
                    "Take": 100,
                    "MemberId": request["member"],
                }
            )
            if operation == "parent":
                params = params.set("InterestIds", request["parent"])
            url = INTERESTS + "/api/v2/Interests"
        else:
            raise SourceError(f"Unknown source operation: {operation}")
        payload = self._get(url, params)
        return {
            "payload": payload,
            "fetched_at": datetime.now(UTC).isoformat(),
            "context": f"{operation} offset {offset}",
        }
