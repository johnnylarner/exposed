"""Register of Interests v2 requests using the shared bounded HTTP retries."""

import httpx

from exposed.api import ParliamentAPI
from exposed.declaration_models import DeclarationPage

INTERESTS_BASE_URL = "https://interests-api.parliament.uk"


class DeclarationsAPI(ParliamentAPI):
    def search_page(
        self, member_id: int, *, skip: int, take: int, interest_id: int | None = None
    ) -> DeclarationPage:
        params = httpx.QueryParams(
            {
                "Type": "Commons",
                "MemberId": member_id,
                "ExcludeExpired": "false",
                "ExpandChildInterests": "false",
                "Skip": skip,
                "Take": take,
            }
        )
        if interest_id is not None:
            params = params.set("InterestIds", interest_id)
        return DeclarationPage.model_validate_json(self._get("/api/v2/Interests", params))
