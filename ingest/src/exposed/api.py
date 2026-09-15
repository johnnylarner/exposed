"""Members API access, bounded retries, complete pagination."""

import logging
import time
from collections.abc import Callable, Iterator
from datetime import UTC, datetime
from email.utils import parsedate_to_datetime
from typing import Any

import httpx

from exposed.models import (
    ImportValidationError,
    MemberHistories,
    MemberSearchPage,
    integer,
    object_value,
    parse_history,
    parse_profile,
)

BASE_URL = "https://members-api.parliament.uk"
PAGE_SIZE = 100
logger = logging.getLogger(__name__)


class APIError(RuntimeError):
    """A required API request failed after bounded retries."""


class MembersAPI:
    def __init__(
        self,
        client: httpx.Client,
        request_delay: float = 0.2,
        sleep: Callable[[float], None] = time.sleep,
    ):
        self.client = client
        self.request_delay = request_delay
        self.sleep = sleep

    def _retry_delay(self, response: httpx.Response | None, attempt: int) -> float:
        if response is not None and (retry_after := response.headers.get("Retry-After")):
            try:
                seconds = float(retry_after)
            except ValueError:
                try:
                    seconds = (
                        parsedate_to_datetime(retry_after) - datetime.now(UTC)
                    ).total_seconds()
                except ValueError, TypeError, OverflowError:
                    seconds = 2**attempt
            return min(30, max(0, seconds))
        return min(30, 2**attempt)

    def _get(self, path: str, params: httpx.QueryParams) -> Any:
        response = None
        for attempt in range(4):
            self.sleep(self.request_delay)
            try:
                response = self.client.get(path, params=params)
            except httpx.TransportError as exc:
                if attempt == 3:
                    raise APIError(f"Members API transport failure at {path}") from exc
                response = None
            else:
                if response.status_code == 200:
                    try:
                        payload = response.json()
                    except ValueError as exc:
                        raise ImportValidationError(f"Invalid JSON from {path}") from exc
                    return payload
                if response.status_code != 429 and response.status_code < 500:
                    raise APIError(f"Members API returned HTTP {response.status_code} at {path}")
                if attempt == 3:
                    raise APIError(f"Members API returned HTTP {response.status_code} at {path}")
            logger.warning("Retrying %s (attempt %s/4)", path, attempt + 2)
            self.sleep(self._retry_delay(response, attempt))
        raise AssertionError("Unreachable retry state")

    def search_pages(
        self,
        filters: dict[str, str | int],
    ) -> Iterator[MemberSearchPage]:
        """Yield validated pages without collecting the whole result in memory."""
        seen: set[int] = set()
        expected = None
        offset = 0
        params = httpx.QueryParams()
        for k, v in [*filters.items(), ("take", PAGE_SIZE)]:
            params = params.set(k, v)

        while True:
            params = params.set("skip", offset)
            response = self._get("/api/Members/Search", params)
            payload = object_value(response, "member search")
            total = integer(payload.get("totalResults"), "totalResults", 0)
            if expected is None:
                expected = total
            if total != expected or payload.get("skip") != offset:
                raise ImportValidationError("Search pagination changed during the import; rerun")
            items = payload.get("items")
            if not isinstance(items, list) or len(items) > PAGE_SIZE:
                raise ImportValidationError("Invalid search page")
            if not items and offset < total:
                raise ImportValidationError(
                    "Search ended before all advertised members were returned"
                )
            members: MemberSearchPage = {}
            for item in items:
                value = object_value(object_value(item, "search item").get("value"), "member")
                member_id = integer(value.get("id"), "member ID")
                if member_id in seen:
                    raise ImportValidationError(f"Duplicate member {member_id} across search pages")
                seen.add(member_id)
                members[member_id] = parse_profile(value)
            offset += len(items)
            if offset > total:
                raise ImportValidationError("Search returned more members than advertised")
            yield members
            if offset == total:
                return

    def histories(
        self,
        member_ids: set[int],
    ) -> MemberHistories:
        """Fetch exactly the histories needed for one member-search page."""
        if not 1 <= len(member_ids) <= PAGE_SIZE:
            raise ValueError(f"History requests need between 1 and {PAGE_SIZE} member IDs")
        params = httpx.QueryParams()
        for member_id in sorted(member_ids):
            params = params.add("ids", member_id)
        response = self._get("/api/Members/History", params)
        if not isinstance(response, list):
            raise ImportValidationError("Invalid history response")
        histories: MemberHistories = {}
        for item in response:
            value = object_value(object_value(item, "history item").get("value"), "history")
            member_id = integer(value.get("id"), "history member ID")
            if member_id in histories:
                raise ImportValidationError(f"Duplicate history for member {member_id}")
            histories[member_id] = parse_history(value)
        if histories.keys() != member_ids:
            raise ImportValidationError(
                "History response does not contain all requested member IDs"
            )
        return histories
