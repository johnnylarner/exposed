"""Fetch typed Parliament responses with bounded retries."""

import logging
import time
from collections.abc import Callable
from datetime import UTC, datetime
from email.utils import parsedate_to_datetime

import httpx

from exposed.models import HistoryBatch, SearchPage

BASE_URL = "https://members-api.parliament.uk"
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

    def _get(self, path: str, params: httpx.QueryParams) -> bytes:
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
                    return response.content
                if response.status_code != 429 and response.status_code < 500:
                    raise APIError(f"Members API returned HTTP {response.status_code} at {path}")
                if attempt == 3:
                    raise APIError(f"Members API returned HTTP {response.status_code} at {path}")
            logger.warning("Retrying %s (attempt %s/4)", path, attempt + 2)
            self.sleep(self._retry_delay(response, attempt))
        raise AssertionError("Unreachable retry state")

    def search_page(
        self,
        filters: dict[str, str | int],
        *,
        skip: int,
        take: int,
    ) -> SearchPage:
        params = httpx.QueryParams(filters).set("skip", skip).set("take", take)
        return SearchPage.from_json(self._get("/api/Members/Search", params))

    def histories(self, member_ids: set[int]) -> HistoryBatch:
        params = httpx.QueryParams()
        for member_id in sorted(member_ids):
            params = params.add("ids", member_id)
        return HistoryBatch.from_json(self._get("/api/Members/History", params))
