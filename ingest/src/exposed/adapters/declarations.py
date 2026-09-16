"""Register of Interests v2 requests using the shared bounded HTTP retries."""

from collections.abc import Iterator, Sequence
from datetime import UTC, datetime

import httpx
from pydantic import ValidationError

from exposed.adapters.declaration_models import DeclarationPage, SourceDeclaration
from exposed.adapters.parliament import BATCH_SIZE, ParliamentAPI
from exposed.core.declarations import DeclarationDraft, RetrievedDeclaration
from exposed.core.errors import (
    DeclarationParseError,
    ImportValidationError,
    validation_error_message,
)

INTERESTS_BASE_URL = "https://interests-api.parliament.uk"


class DeclarationsAPI(ParliamentAPI):
    def search_page(
        self,
        member_id: int | None,
        *,
        skip: int,
        take: int,
        interest_id: int | None = None,
        interest_ids: Sequence[int] = (),
    ) -> DeclarationPage:
        params = httpx.QueryParams(
            {
                "Type": "Commons",
                "ExcludeExpired": "false",
                "ExpandChildInterests": "false",
                "Skip": skip,
                "Take": take,
            }
        )
        if member_id is not None:
            params = params.set("MemberId", member_id)
        if interest_id is not None:
            params = params.set("InterestIds", interest_id)
        for source_id in interest_ids:
            params = params.add("InterestIds", source_id)
        return DeclarationPage.model_validate_json(self._get("/api/v2/Interests", params))

    def declarations_by_ids(self, source_ids: Sequence[int]) -> Iterator[DeclarationPage]:
        """Fetch existing IDs in bounded requests without member or parent traversal."""
        for start in range(0, len(source_ids), BATCH_SIZE):
            ids = source_ids[start : start + BATCH_SIZE]
            offset = 0
            while True:
                page = self.search_page(None, skip=offset, take=BATCH_SIZE, interest_ids=ids)
                yield page
                offset += len(page.items)
                if not page.items or offset >= page.total_results:
                    break

    def declarations(self, member_id: int) -> Iterator[tuple[RetrievedDeclaration, ...]]:
        """Traverse all registers and dates, allowing changing totals and empty final pages."""
        offset = 0
        try:
            while True:
                page = self.search_page(member_id, skip=offset, take=BATCH_SIZE)
                yield self._records(page, f"offset {offset}")
                offset += len(page.items)
                if not page.items or offset >= page.total_results:
                    return
        except ValidationError as exc:
            raise ImportValidationError(validation_error_message(exc)) from exc

    def parent_declarations(
        self,
        member_id: int,
        parent_id: int,
    ) -> tuple[RetrievedDeclaration, ...]:
        try:
            page = self.search_page(member_id, skip=0, take=BATCH_SIZE, interest_id=parent_id)
            return self._records(page, f"parent {parent_id}")
        except ValidationError as exc:
            raise ImportValidationError(validation_error_message(exc)) from exc

    def _records(self, page: DeclarationPage, context: str) -> tuple[RetrievedDeclaration, ...]:
        fetched_at = datetime.now(UTC)
        return tuple(
            RetrievedDeclaration(
                identifier=raw.get("id") if isinstance(raw, dict) else None,
                payload=raw,
                fetched_at=fetched_at,
                context=context,
            )
            for raw in page.items
        )

    def interpret(self, record: RetrievedDeclaration) -> DeclarationDraft:
        try:
            return SourceDeclaration.model_validate(record.payload).to_draft()
        except ValidationError as exc:
            raise DeclarationParseError(parsing_error(exc)) from exc


def parsing_error(exc: ValidationError | DeclarationParseError) -> str:
    if isinstance(exc, DeclarationParseError):
        return str(exc)
    return "; ".join(
        f"{'.'.join(str(part) for part in issue['loc']) or 'declaration'}: "
        f"{issue['msg']}; input={issue.get('input')!r}"
        for issue in exc.errors(include_url=False, include_context=False)
    )
