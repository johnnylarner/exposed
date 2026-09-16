import httpx
import pytest

from exposed.core.errors import DeclarationParseError, ImportValidationError, SourceError
from tests.declaration_fakes import DeclarationsFixture, declaration, money


def test_source_port_preserves_evidence_and_isolates_bad_item_decoding():
    good, bad = declaration(), declaration(102, fields=[money("janky")])
    fixture = DeclarationsFixture(good, bad)
    api = fixture.api()
    with api.client:
        (batch,) = api.declarations(1)
        assert [record.payload for record in batch] == [good, bad]
        assert api.interpret(batch[0]).accept().id == 101
        with pytest.raises(DeclarationParseError, match="janky") as error:
            api.interpret(batch[1])
    assert "Value.value" in str(error.value)


@pytest.mark.parametrize(
    "failure, error_type", [(503, SourceError), ("bad-page", ImportValidationError)]
)
def test_source_port_translates_dependency_and_envelope_errors(failure, error_type):
    fixture = DeclarationsFixture()
    fixture.override = lambda _: (
        httpx.Response(503) if failure == 503 else httpx.Response(200, json={})
    )
    api = fixture.api()
    with api.client, pytest.raises(error_type) as error:
        list(api.declarations(1))
    if failure == "bad-page":
        assert error.value.__cause__ is not None


def test_preferred_name_preserves_ignored_malformed_fallback_compatibility():
    from tests.declaration_fakes import field

    fixture = DeclarationsFixture(
        declaration(
            fields=[
                money("100"),
                field("UltimatePayerName", "Ultimate payer"),
                field("DonorName", {"invalid": "unused fallback"}),
            ]
        )
    )
    api = fixture.api()
    with api.client:
        (batch,) = api.declarations(1)
        assert api.interpret(batch[0]).accept().funding[0].funder == "Ultimate payer"
