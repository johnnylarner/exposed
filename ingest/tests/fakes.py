"""Synthetic Parliament responses; these are not real people or service records."""

from collections.abc import Callable
from copy import deepcopy
from datetime import date
from typing import Any

import httpx

from exposed.api import BASE_URL, MembersAPI

TERM_START = date(2024, 7, 4)
AS_OF = date(2026, 9, 15)


def service(start: str = "2024-07-04", end: str | None = None, house: int = 1) -> dict[str, Any]:
    return {
        "house": house,
        "membershipStartDate": f"{start}T00:00:00",
        "membershipEndDate": None if end is None else f"{end}T00:00:00",
        "membershipFrom": "Example constituency",
    }


class ParliamentFixture:
    def __init__(self, count: int = 3):
        self.profiles: dict[int, dict[str, Any]] = {
            i: {
                "id": i,
                "nameDisplayAs": f"Example Member {i}",
                "latestParty": {"id": 1, "name": "Example party"},
                "latestHouseMembership": service(start="1987-06-11"),
                "unknownFutureField": {"retained": True},
            }
            for i in range(1, count + 1)
        }
        self.histories: dict[int, dict[str, Any]] = {
            i: {
                "id": i,
                "houseMembershipHistory": [
                    service("2019-12-12", "2024-05-30"),
                    service(),
                ],
            }
            for i in self.profiles
        }
        self.current = set(self.profiles)
        self.requests: list[httpx.Request] = []
        self.sleeps: list[float] = []
        self.override: Callable[[httpx.Request], httpx.Response | None] | None = None

    def leave(self, member_id: int, end: str = "2025-03-17", *, lords: bool = False) -> None:
        self.current.remove(member_id)
        self.histories[member_id]["houseMembershipHistory"][-1]["membershipEndDate"] = (
            f"{end}T00:00:00"
        )
        self.profiles[member_id]["latestHouseMembership"]["membershipEndDate"] = f"{end}T00:00:00"
        if lords:
            self.profiles[member_id]["latestHouseMembership"] = service("2025-04-01", house=2)
            self.histories[member_id]["houseMembershipHistory"].append(
                service("2025-04-01", house=2)
            )

    def handle(self, request: httpx.Request) -> httpx.Response:
        self.requests.append(request)
        if self.override:
            response = self.override(request)
            if response is not None:
                return response
        if request.url.path.endswith("/History"):
            ids = [int(i) for i in request.url.params.get_list("ids")]
            return httpx.Response(200, json=[{"value": deepcopy(self.histories[i])} for i in ids])
        assert request.url.path.endswith("/Search")
        if request.url.params.get("IsCurrentMember") == "true":
            assert request.url.params["House"] == "1"
            ids = sorted(self.current)
        else:
            # Latest House/current/eligibility filters would silently drop former MPs.
            assert "House" not in request.url.params
            assert "IsCurrentMember" not in request.url.params
            assert "IsEligible" not in request.url.params
            assert request.url.params["MembershipInDateRange.WasMemberOfHouse"] == "1"
            ids = sorted(self.profiles)
        skip = int(request.url.params["skip"])
        take = int(request.url.params["take"])
        return httpx.Response(
            200,
            json={
                "items": [{"value": deepcopy(self.profiles[i])} for i in ids[skip : skip + take]],
                "totalResults": len(ids),
                "skip": skip,
                "take": take,
            },
        )

    def api(self) -> MembersAPI:
        return MembersAPI(
            httpx.Client(base_url=BASE_URL, transport=httpx.MockTransport(self.handle)),
            request_delay=0,
            sleep=self.sleeps.append,
        )
