"""Synthetic Interests v2 payloads following the official nested field contract."""

from collections.abc import Callable
from copy import deepcopy
from typing import Any

import httpx

from exposed.declaration_api import INTERESTS_BASE_URL, DeclarationsAPI


def field(name: str, value: Any, kind: str = "String", **extra: Any) -> dict[str, Any]:
    return {"name": name, "type": kind, "typeInfo": None, "value": value, **extra}


def money(value: Any = "2000.00", currency: str | None = "GBP") -> dict[str, Any]:
    return field("Value", value, "Decimal", typeInfo={"currencyCode": currency})


def declaration(id: int = 101, member: int = 1, fields=None) -> dict[str, Any]:
    return {
        "id": id,
        "parentInterestId": None,
        "category": {"id": 9, "name": "Miscellaneous", "type": "Commons"},
        "registrant": {"type": "Member", "memberDetail": {"id": member}},
        "childInterests": [],
        "versions": [
            {
                "register": {"id": 729, "publishedDate": "2024-08-04", "type": "Commons"},
                "publishedDate": "2016-02-01",
                "registrationDate": "2016-01-27",
                "fields": fields
                if fields is not None
                else [field("Description", "Unpaid trustee")],
            }
        ],
        "unknownFutureField": {"retained": True},
    }


class DeclarationsFixture:
    def __init__(self, *items: dict[str, Any]):
        self.items = list(items)
        self.requests: list[httpx.Request] = []
        self.override: Callable[[httpx.Request], httpx.Response | None] | None = None

    def handle(self, request: httpx.Request) -> httpx.Response:
        self.requests.append(request)
        if self.override:
            response = self.override(request)
            if response is not None:
                return response
        assert request.url.path == "/api/v2/Interests"
        params = request.url.params
        assert params["Type"] == "Commons"
        assert params["ExcludeExpired"] == "false"
        assert params["ExpandChildInterests"] == "false"
        assert not {"RegisterId", "CategoryId", "PublishedFrom", "UpdatedFrom"} & set(params)
        items = [
            i
            for i in self.items
            if "MemberId" not in params
            or i["registrant"]["memberDetail"]["id"] == int(params["MemberId"])
        ]
        if "InterestIds" in params:
            ids = {int(value) for value in params.get_list("InterestIds")}
            items = [i for i in items if i["id"] in ids]
        skip, take = int(params["Skip"]), int(params["Take"])
        return httpx.Response(
            200,
            json={
                "items": deepcopy(items[skip : skip + take]),
                "totalResults": len(items),
                "skip": skip,
                "take": take,
            },
        )

    def api(self) -> DeclarationsAPI:
        return DeclarationsAPI(
            httpx.Client(base_url=INTERESTS_BASE_URL, transport=httpx.MockTransport(self.handle)),
            request_delay=0,
            sleep=lambda _: None,
        )
