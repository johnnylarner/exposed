"""Compatibility facade for imports used before the ports-and-adapters migration."""

import psycopg

from exposed.adapters.parliament import (
    BATCH_SIZE as BATCH_SIZE,
)
from exposed.adapters.parliament import (
    MembersAPI,
    validate_history_request,
)
from exposed.adapters.parliament import (
    search_pages as search_pages,
)
from exposed.adapters.parliament_models import MemberHistory
from exposed.adapters.postgres import connect as connect
from exposed.adapters.postgres import storage_error
from exposed.composition import run_import as run_import
from exposed.core.errors import ImportFailed as ImportFailed
from exposed.core.errors import safe_error as core_safe_error
from exposed.core.refresh import match_histories


def safe_error(exc: BaseException) -> str:
    """Retain safe formatting of raw driver exceptions for existing callers."""
    if isinstance(exc, psycopg.Error):
        return str(storage_error(exc))
    return core_safe_error(exc)


def load_histories(api: MembersAPI, member_ids: set[int]) -> dict[int, MemberHistory]:
    """Legacy history-batch interface, retaining its input-size and parsing errors."""
    validate_history_request(member_ids)
    return match_histories(member_ids, api.histories(member_ids).histories)
