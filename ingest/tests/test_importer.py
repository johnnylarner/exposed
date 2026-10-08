from datetime import date

import pytest

from exposed.importer import ImportFailed, run_import
from tests.fakes import AS_OF, TERM_START, ParliamentFixture

pytestmark = pytest.mark.integration


def run(url: str, fixture: ParliamentFixture, term_start: date = TERM_START) -> dict[str, object]:
    api = fixture.api()
    with api.client:
        return run_import(url, term_start, api, as_of=AS_OF)



def test_future_term_start_is_rejected_before_api_fetch(database_url):
    fixture = ParliamentFixture()
    with pytest.raises(ImportFailed, match="after the import date"):
        run(database_url, fixture, date(2030, 1, 1))
    assert fixture.requests == []
