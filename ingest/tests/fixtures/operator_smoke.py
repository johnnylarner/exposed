"""Real operator/Rust round trip, with only Parliament's network substituted."""

import json
import sys
import time

import httpx

from exposed.operator import run_import
from exposed.source import SourceClient


def parliament(request):
    if request.url.path.endswith("History"):
        payload = [
            {
                "value": {
                    "id": 1,
                    "houseMembershipHistory": [{"house": 1, "membershipStartDate": "2024-07-04"}],
                }
            }
        ]
    elif request.url.path.endswith("Search"):
        payload = {
            "items": [
                {
                    "value": {
                        "id": 1,
                        "nameDisplayAs": "Example Member",
                        "latestParty": {"id": 1, "name": "Party"},
                        "latestHouseMembership": {"house": 1, "membershipFrom": "Seat"},
                    }
                }
            ],
            "skip": 0,
            "totalResults": 1,
        }
    else:
        payload = {
            "items": [
                {
                    "id": 101,
                    "category": {"id": 1, "name": "Employment", "type": "Commons"},
                    "registrant": {"type": "Member", "memberDetail": {"id": 1}},
                    "versions": [
                        {
                            "register": {"publishedDate": "2026-09-01"},
                            "fields": [
                                {"name": "DonorName", "value": "Example Limited"},
                                {"name": "Value", "type": "Decimal", "value": "12.50"},
                            ],
                        }
                    ],
                }
            ],
            "skip": 0,
            "totalResults": 1,
        }
    return httpx.Response(200, json=payload)


with httpx.Client(transport=httpx.MockTransport(parliament)) as api:
    with httpx.Client(base_url=sys.argv[1], timeout=20) as app:
        source = SourceClient(api, sleep=lambda _: None)
        run_import("initialize", "2024-07-04", app, source)
        time.sleep(0.05)  # Allow the previous dedicated lock connection to close.
        print(json.dumps(run_import("initialize", "2024-07-04", app, source)))
