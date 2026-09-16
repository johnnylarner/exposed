"""Fetch typed Parliament responses with bounded retries."""

import logging
import time
from collections.abc import Callable, Iterator
from datetime import UTC, date, datetime
from email.utils import parsedate_to_datetime

import httpx
from pydantic import ValidationError

from exposed.adapters.parliament_models import HistoryBatch, SearchPage
from exposed.core.errors import ImportValidationError, SourceError, validation_error_message
from exposed.core.models import MemberHistory, MemberProfile

BASE_URL = "https://members-api.parliament.uk"
logger = logging.getLogger(__name__)


APIError = SourceError
BATCH_SIZE = 100


class ParliamentAPI:
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
                    raise APIError(f"Parliament API transport failure at {path}") from exc
                response = None
            else:
                if response.status_code == 200:
                    return response.content
                if response.status_code != 429 and response.status_code < 500:
                    raise APIError(f"Parliament API returned HTTP {response.status_code} at {path}")
                if attempt == 3:
                    raise APIError(f"Parliament API returned HTTP {response.status_code} at {path}")
            logger.warning("Retrying %s (attempt %s/4)", path, attempt + 2)
            self.sleep(self._retry_delay(response, attempt))
        raise AssertionError("Unreachable retry state")


class MembersAPI(ParliamentAPI):
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

    def current_commons(self) -> Iterator[MemberProfile]:
        try:
            for page in search_pages(self, {"House": 1, "IsCurrentMember": "true"}):
                yield from page.members
        except ValidationError as exc:
            raise ImportValidationError(validation_error_message(exc)) from exc

    def commons_candidates(
        self, term_start: date, as_of: date
    ) -> Iterator[tuple[MemberProfile, ...]]:
        try:
            for page in search_pages(
                self,
                {
                    "MembershipInDateRange.WasMemberOnOrAfter": term_start.isoformat(),
                    "MembershipInDateRange.WasMemberOnOrBefore": as_of.isoformat(),
                    "MembershipInDateRange.WasMemberOfHouse": 1,
                },
            ):
                yield page.members
        except ValidationError as exc:
            raise ImportValidationError(validation_error_message(exc)) from exc

    def member_histories(self, member_ids: set[int]) -> tuple[MemberHistory, ...]:
        validate_history_request(member_ids)
        try:
            return tuple(history.to_history() for history in self.histories(member_ids).histories)
        except ValidationError as exc:
            raise ImportValidationError(validation_error_message(exc)) from exc


def validate_history_request(member_ids: set[int]) -> None:
    if not 1 <= len(member_ids) <= BATCH_SIZE:
        raise ImportValidationError(f"History requests need between 1 and {BATCH_SIZE} member IDs")


def search_pages(api: MembersAPI, filters: dict[str, str | int]) -> Iterator[SearchPage]:
    """Advance by the number returned until the API's reported total is reached."""
    offset = 0
    while True:
        page = api.search_page(filters, skip=offset, take=BATCH_SIZE)
        if not page.members and offset < page.total_results:
            raise ImportValidationError("Search ended before all advertised members were returned")
        yield page
        offset += len(page.members)
        if offset >= page.total_results:
            return
