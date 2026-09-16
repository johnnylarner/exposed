from datetime import UTC, date, datetime

import pytest

from exposed.declaration_importer import run_import
from exposed.importer import connect
from tests.declaration_fakes import DeclarationsFixture, declaration
from tests.fakes import TERM_START, ParliamentFixture, service
from tests.test_importer import dataset as member_dataset
from tests.test_importer import run as import_members

pytestmark = pytest.mark.integration


def run(url: str, fixture: DeclarationsFixture):
    api = fixture.api()
    with api.client:
        return run_import(url, TERM_START, api)


def dataset(url: str):
    with connect(url) as conn:
        return {
            "declarations": conn.execute(
                "SELECT * FROM exposed.declarations ORDER BY source_declaration_id"
            ).fetchall(),
            "funding": conn.execute("SELECT * FROM exposed.funding_entries ORDER BY id").fetchall(),
        }


def test_imports_stored_cohort_once_and_stores_only_parsed_declaration_fields(database_url):
    members = ParliamentFixture(2)
    members.leave(2, lords=True)
    members.histories[1]["houseMembershipHistory"] = [
        service(end="2025-01-01"),
        service("2025-03-01"),
    ]
    import_members(database_url, members)
    before = member_dataset(database_url)
    fixture = DeclarationsFixture(declaration(), declaration(102, member=2))
    started = datetime.now(UTC)

    result = run(database_url, fixture)

    assert result["status"] == "succeeded"
    data = dataset(database_url)
    assert len(data["declarations"]) == 2
    assert data["funding"] == []
    assert [r.url.params["MemberId"] for r in fixture.requests] == ["1", "2"]
    member_ids = {r["parliament_member_id"]: r["id"] for r in before["members"]}
    for row, source in zip(data["declarations"], fixture.items, strict=True):
        assert row["id"].version == 7
        assert set(row) == {
            "id",
            "source_declaration_id",
            "member_id",
            "category_id",
            "category_name",
            "fetched_at",
            "registration_date",
        }
        assert row["source_declaration_id"] == source["id"]
        assert row["category_name"] == "Miscellaneous"
        assert row["member_id"] == member_ids[source["registrant"]["memberDetail"]["id"]]
        assert started <= row["fetched_at"] <= datetime.now(UTC)
        assert row["registration_date"] == date(2016, 1, 27)
    assert member_dataset(database_url) == before


def test_extracts_latest_direct_in_kind_and_nested_funding(database_url):
    from copy import deepcopy
    from decimal import Decimal

    from tests.declaration_fakes import field, money

    import_members(database_url, ParliamentFixture(1))
    direct = declaration(
        fields=[
            field("DonorName", "Example donor"),
            money("2000.01"),
            field("PaymentType", "Monetary"),
            field("HoursWorked", "9.5", "Decimal"),
        ]
    )
    direct["category"] = {"id": 3, "name": "Donations and other support", "type": "Commons"}
    older = deepcopy(direct["versions"][0])
    older["register"] = {"id": 9999, "publishedDate": "2023-01-01", "type": "Commons"}
    older["fields"][1]["value"] = "999.00"
    direct["versions"].append(older)
    visit = declaration(
        102,
        fields=[
            field(
                "Donors",
                None,
                "Donor[]",
                values=[
                    [
                        field("Name", "Example council"),
                        money("10230.00"),
                        field("PaymentType", "In kind"),
                    ],
                    [field("Name", "Example ministry"), money("320.125", "EUR")],
                ],
            )
        ],
    )
    fixture = DeclarationsFixture(direct, visit)

    run(database_url, fixture)

    rows = dataset(database_url)["funding"]
    assert {
        (r["source_declaration_id"], r["funder"], r["amount"], r["currency"], r["payment_type"])
        for r in rows
    } == {
        (101, "Example donor", Decimal("2000.01"), "GBP", "Monetary"),
        (102, "Example council", Decimal("10230"), "GBP", "In kind"),
        (102, "Example ministry", Decimal("320.125"), "EUR", None),
    }
    assert all(r["id"].version == 7 for r in rows)
    assert (
        dataset(database_url)["declarations"][0]["category_name"] == "Donations and other support"
    )


def test_refresh_preserves_unchanged_groups_and_replaces_changed_groups(database_url):
    from copy import deepcopy
    from decimal import Decimal

    from tests.declaration_fakes import field, money

    import_members(database_url, ParliamentFixture(1))
    groups = [
        [field("Name", "Same donor"), money("12.50")],
        [field("Name", "Same donor"), money("12.50")],
        [field("Name", "Other donor"), money("25")],
    ]
    source = declaration(fields=[field("Donors", None, "Donor[]", values=groups)])
    fixture = DeclarationsFixture(source)
    run(database_url, fixture)
    before = dataset(database_url)
    run(database_url, fixture)
    repeated = dataset(database_url)
    assert repeated["funding"] == before["funding"]
    assert repeated["declarations"][0]["id"] == before["declarations"][0]["id"]
    assert repeated["declarations"][0]["fetched_at"] >= before["declarations"][0]["fetched_at"]
    groups.reverse()
    source["unknownFutureField"] = {"changed": True}
    source["category"]["name"] = "Updated category"
    source["versions"][0]["registrationDate"] = "2016-01-28"
    run(database_url, fixture)
    assert dataset(database_url)["funding"] == before["funding"]
    assert dataset(database_url)["declarations"][0]["category_name"] == "Updated category"
    assert dataset(database_url)["declarations"][0]["registration_date"] == date(2016, 1, 28)
    groups.pop(0)
    groups[0][1]["value"] = "99.01"
    run(database_url, fixture)
    changed = dataset(database_url)
    assert {r["amount"] for r in changed["funding"]} == {Decimal("12.5"), Decimal("99.01")}
    assert not {r["id"] for r in changed["funding"]} & {r["id"] for r in before["funding"]}
    assert changed["declarations"][0]["id"] == before["declarations"][0]["id"]
    # A corrected parser must repair an old projection even with identical source JSON.
    source_before = deepcopy(source)
    with connect(database_url) as conn:
        conn.execute("UPDATE exposed.funding_entries SET amount = 0")
    run(database_url, fixture)
    assert source == source_before
    assert {r["amount"] for r in dataset(database_url)["funding"]} == {
        Decimal("12.5"),
        Decimal("99.01"),
    }


def test_bad_second_funder_rejects_entire_update_and_new_item_but_commits_siblings(
    database_url, caplog
):
    from copy import deepcopy

    from tests.declaration_fakes import field, money

    import_members(database_url, ParliamentFixture(1))
    source = declaration(
        fields=[
            field(
                "Donors",
                None,
                "Donor[]",
                values=[
                    [field("Name", "First"), money("100")],
                    [field("Name", "Second"), money("200")],
                ],
            )
        ]
    )
    fixture = DeclarationsFixture(source)
    run(database_url, fixture)
    before = dataset(database_url)
    source["versions"][0]["fields"][0]["values"][1][1]["value"] = "about £200"
    invalid_new = deepcopy(source)
    invalid_new["id"] = 102
    fixture.items = [declaration(103), source, invalid_new, declaration(104)]

    result = run(database_url, fixture)

    after = dataset(database_url)
    assert result["status"] == "succeeded"
    assert [r["source_declaration_id"] for r in after["declarations"]] == [101, 103, 104]
    assert after["declarations"][0] == before["declarations"][0]
    assert after["funding"] == before["funding"]
    for id in [101, 102]:
        assert f"declaration {id}" in caplog.text
    assert "Donors.values.1.1.Value.value" in caplog.text
    assert "about £200" in caplog.text
    assert "unsupported amount" in caplog.text


def test_child_payment_resolves_parent_payer_and_prefers_explicit_ultimate_payer(database_url):
    from tests.declaration_fakes import field, money

    import_members(database_url, ParliamentFixture(1))
    child = declaration(101, fields=[money("340"), field("HoursWorked", "2", "Decimal")])
    child["parentInterestId"] = 500
    parent = declaration(500, fields=[field("PayerName", "Example publisher")])
    explicit = declaration(
        102,
        fields=[
            money("18450"),
            field("PayerName", "Intermediary publisher"),
            field("DonorName", "Direct donor"),
            field("UltimatePayerName", "Ultimate publisher"),
        ],
    )
    explicit["parentInterestId"] = 501
    fixture = DeclarationsFixture(child, explicit, parent)

    run(database_url, fixture)

    data = dataset(database_url)
    assert {r["source_declaration_id"]: r["funder"] for r in data["funding"]} == {
        101: "Example publisher",
        102: "Ultimate publisher",
    }
    assert len(data["declarations"]) == 3
    assert all("InterestIds" not in r.url.params for r in fixture.requests)


@pytest.mark.parametrize("conflicting", [False, True])
def test_duplicates_are_reported_or_fail_the_refresh(database_url, caplog, conflicting):
    from copy import deepcopy

    from exposed.importer import ImportFailed

    import_members(database_url, ParliamentFixture(1))
    original = declaration()
    fixture = DeclarationsFixture(original)
    run(database_url, fixture)
    before = dataset(database_url)
    original["unknownFutureField"] = "Updated in first page"
    repeated = deepcopy(original)
    if conflicting:
        repeated["unknownFutureField"] = "Conflicting later page"
    fixture.items = [original, *(declaration(i) for i in range(102, 201)), repeated]

    if conflicting:
        with pytest.raises(ImportFailed, match="Conflicting.*101"):
            run(database_url, fixture)
        assert dataset(database_url) == before
    else:
        result = run(database_url, fixture)
        assert result["declarations"] == 100
        assert len(dataset(database_url)["declarations"]) == 100
        assert "Duplicate declaration 101" in caplog.text


@pytest.mark.parametrize(
    "bad_fields",
    [
        [
            {
                "name": "FutureAmount",
                "type": "Decimal",
                "typeInfo": {"currencyCode": "GBP"},
                "value": "20",
            }
        ],
        [
            {
                "name": "OtherPayments",
                "values": [[{"name": "Value", "type": "Decimal", "value": "20"}]],
            }
        ],
        [{"name": "Donors", "type": "Donor[]", "values": None}],
        [{"name": "Donors", "type": "Donor[]", "values": [[]]}],
    ],
)
def test_uninterpretable_funding_is_rejected_not_treated_as_nonfinancial(
    database_url, caplog, bad_fields
):
    import_members(database_url, ParliamentFixture(1))
    fixture = DeclarationsFixture(declaration(fields=bad_fields), declaration(102))
    run(database_url, fixture)
    assert [r["source_declaration_id"] for r in dataset(database_url)["declarations"]] == [102]
    assert "Rejected declaration 101" in caplog.text


@pytest.mark.parametrize("failure", ["http", "page", "database", "interrupt"])
def test_late_failure_keeps_completed_member_and_rolls_back_current_member(database_url, failure):
    import httpx

    from exposed.importer import ImportFailed
    from tests.declaration_fakes import field, money

    import_members(database_url, ParliamentFixture(2))
    fixture = DeclarationsFixture(
        *(
            declaration(i, member=2, fields=[field("DonorName", "Donor"), money()])
            for i in range(101, 202)
        )
    )
    run(database_url, fixture)
    before = dataset(database_url)
    members_before = member_dataset(database_url)
    fixture.items[0]["versions"][0]["fields"][1]["value"] = "1"
    fixture.items[-1]["versions"][0]["fields"][1]["value"] = "999"
    fixture.items.insert(0, declaration(99, member=1))
    if failure == "database":
        with connect(database_url) as conn:
            conn.execute("ALTER TABLE exposed.funding_entries ADD CHECK (amount <> 999) NOT VALID")
    observed = []
    committed = []

    def fail_late(request):
        if request.url.params["MemberId"] == "2" and request.url.params["Skip"] == "0":
            committed.append(dataset(database_url))
        if request.url.params["Skip"] == "100":
            observed.append(dataset(database_url))
            if failure == "http":
                return httpx.Response(503)
            if failure == "page":
                return httpx.Response(200, json={"items": []})
            if failure == "interrupt":
                raise KeyboardInterrupt
        return None

    fixture.override = fail_late
    with pytest.raises(ImportFailed) as error:
        run(database_url, fixture)
    assert error.value.interrupted == (failure == "interrupt")
    assert committed[0]["declarations"][0]["source_declaration_id"] == 99
    assert committed[0]["declarations"][1:] == before["declarations"]
    assert committed[0]["funding"] == before["funding"]
    assert observed and all(data == committed[0] for data in observed)
    assert dataset(database_url) == committed[0]
    assert member_dataset(database_url) == members_before
    if failure == "http":
        assert len(observed) == 4


def test_invalid_registration_date_preserves_previous_record_and_accepts_siblings(
    database_url, caplog
):
    import_members(database_url, ParliamentFixture(1))
    source = declaration()
    fixture = DeclarationsFixture(source)
    run(database_url, fixture)
    before = dataset(database_url)
    source["versions"][0]["registrationDate"] = "not a date"
    missing = declaration(102)
    del missing["versions"][0]["registrationDate"]
    fixture.items.append(missing)
    run(database_url, fixture)
    after = dataset(database_url)
    assert after["declarations"][0] == before["declarations"][0]
    assert after["declarations"][1]["registration_date"] is None
    assert "registrationDate" in caplog.text
    assert "not a date" in caplog.text


def test_pagination_tolerates_changed_totals_empty_pages_and_omitted_declarations(database_url):
    import httpx

    import_members(database_url, ParliamentFixture(1))
    fixture = DeclarationsFixture(declaration())
    run(database_url, fixture)
    before = dataset(database_url)
    responses = iter(
        [
            {"items": [declaration(102)], "totalResults": 8, "skip": 0},
            {"items": [declaration(103)], "totalResults": 6, "skip": 1},
            {"items": [], "totalResults": 10, "skip": 2},
        ]
    )
    fixture.override = lambda _: httpx.Response(200, json=next(responses))
    result = run(database_url, fixture)
    after = dataset(database_url)
    assert result["status"] == "succeeded"
    assert [r["source_declaration_id"] for r in after["declarations"]] == [101, 102, 103]
    assert after["declarations"][0] == before["declarations"][0]
    assert [r.url.params["Skip"] for r in fixture.requests[-3:]] == ["0", "1", "2"]


def test_latest_version_ambiguity_rejects_only_its_declaration(database_url, caplog):
    from copy import deepcopy
    from decimal import Decimal

    from tests.declaration_fakes import money

    import_members(database_url, ParliamentFixture(1))
    ambiguous = declaration(fields=[money("100")])
    other = deepcopy(ambiguous["versions"][0])
    other["fields"][0]["value"] = "200"
    ambiguous["versions"].append(other)
    valid = declaration(102, fields=[money("7")])
    older = deepcopy(valid["versions"][0])
    older["fields"][0]["value"] = "invalid old amount"
    older["register"]["publishedDate"] = "2020-01-01"
    valid["versions"].insert(0, older)
    fixture = DeclarationsFixture(ambiguous, valid)
    run(database_url, fixture)
    assert [r["source_declaration_id"] for r in dataset(database_url)["declarations"]] == [102]
    assert dataset(database_url)["funding"][0]["amount"] == Decimal("7")
    assert "conflicting versions" in caplog.text
    valid["versions"].reverse()
    rows_before = dataset(database_url)["funding"]
    run(database_url, fixture)
    assert dataset(database_url)["funding"] == rows_before


def test_absent_values_remain_absent_and_unrelated_numbers_are_not_funding(database_url):
    from tests.declaration_fakes import field, money

    import_members(database_url, ParliamentFixture(1))
    fixture = DeclarationsFixture(
        declaration(fields=[money(None, None)]),
        declaration(102, fields=[field("HoursWorked", "3.5", "Decimal")]),
    )
    run(database_url, fixture)
    rows = dataset(database_url)["funding"]
    assert len(rows) == 1
    assert (
        rows[0]["funder"]
        is rows[0]["amount"]
        is rows[0]["currency"]
        is rows[0]["payment_type"]
        is None
    )


@pytest.mark.parametrize("relationship", ["later_page", "missing", "cycle", "malformed"])
def test_required_parent_is_resolved_before_accepting_child(database_url, caplog, relationship):
    import httpx

    from tests.declaration_fakes import field, money

    import_members(database_url, ParliamentFixture(1))
    child = declaration(fields=[money()])
    child["parentInterestId"] = 500
    parent = declaration(500, fields=[field("PayerName", "Publisher")])
    if relationship == "cycle":
        parent["parentInterestId"] = 101
        parent["versions"][0]["fields"] = [money()]
    elif relationship == "malformed":
        parent["versions"][0]["fields"] = [field("PayerName", {"bad": "name"})]
    fixture = DeclarationsFixture(child, declaration(102))
    if relationship != "missing":
        fixture.items.extend([*(declaration(i) for i in range(103, 201)), parent])
    run(database_url, fixture)
    data = dataset(database_url)
    if relationship == "later_page":
        assert data["funding"][0]["funder"] == "Publisher"
        assert any(r.url.params.get("InterestIds") == "500" for r in fixture.requests)
    else:
        assert 101 not in {r["source_declaration_id"] for r in data["declarations"]}
        assert "Rejected declaration 101" in caplog.text
    # Required-parent transport failures fail the whole refresh, rather than skipping a child.
    fixture.override = lambda r: httpx.Response(503) if "InterestIds" in r.url.params else None
    from exposed.importer import ImportFailed

    with pytest.raises(ImportFailed, match="503"):
        run(database_url, fixture)


def test_invalid_identity_is_logged_with_member_context_and_valid_siblings_survive(
    database_url, caplog
):
    import httpx

    import_members(database_url, ParliamentFixture(1))
    source = declaration()
    source["id"] = "not-an-id"
    fixture = DeclarationsFixture()
    fixture.override = lambda _: httpx.Response(
        200, json={"items": [source, None, declaration(102)], "skip": 0, "totalResults": 3}
    )
    run(database_url, fixture)
    assert [r["source_declaration_id"] for r in dataset(database_url)["declarations"]] == [102]
    assert "member 1" in caplog.text
    assert "not-an-id" in caplog.text
    assert "id: Input should be a valid integer" in caplog.text


def test_schema_constraints_preserve_members(database_url):
    import psycopg

    from tests.declaration_fakes import money

    import_members(database_url, ParliamentFixture(1))
    members_before = member_dataset(database_url)
    run(database_url, DeclarationsFixture(declaration(fields=[money()])))
    with connect(database_url) as conn:
        with pytest.raises(psycopg.errors.ForeignKeyViolation):
            conn.execute(
                "UPDATE exposed.declarations SET member_id = '00000000-0000-0000-0000-000000000000'"
            )
        with pytest.raises(psycopg.errors.ForeignKeyViolation):
            conn.execute("UPDATE exposed.funding_entries SET source_declaration_id = 9999")
        with pytest.raises(psycopg.errors.UniqueViolation):
            conn.execute("""INSERT INTO exposed.declarations (
                       id, source_declaration_id, member_id, category_id,
                       category_name, fetched_at)
                SELECT '00000000-0000-0000-0000-000000000000', source_declaration_id,
                       member_id, category_id, category_name, fetched_at
                FROM exposed.declarations""")
    assert member_dataset(database_url) == members_before


def test_requires_a_matching_stored_cohort_without_creating_members_or_terms(database_url):
    from datetime import date

    from exposed.importer import ImportFailed

    fixture = DeclarationsFixture(declaration())
    with pytest.raises(ImportFailed, match="No stored Commons cohort"):
        run(database_url, fixture)
    assert all(not rows for rows in member_dataset(database_url).values())
    import_members(database_url, ParliamentFixture(1))
    before = member_dataset(database_url)
    api = fixture.api()
    with api.client, pytest.raises(ImportFailed, match="No stored Commons cohort"):
        run_import(database_url, date(2020, 1, 1), api)
    assert member_dataset(database_url) == before
    assert fixture.requests == []


def test_nested_funding_inside_a_donor_is_not_silently_lost(database_url, caplog):
    from tests.declaration_fakes import field, money

    import_members(database_url, ParliamentFixture(1))
    source = declaration(
        fields=[
            field(
                "Donors",
                None,
                "Donor[]",
                values=[
                    [
                        field("Name", "Top donor"),
                        field(
                            "Donors",
                            None,
                            "Donor[]",
                            values=[[field("Name", "Nested donor"), money("200")]],
                        ),
                    ]
                ],
            )
        ]
    )
    run(database_url, DeclarationsFixture(source))
    assert dataset(database_url) == {"declarations": [], "funding": []}
    assert "unsupported funding" in caplog.text


def test_duplicate_comparison_distinguishes_boolean_and_numeric_source_values(database_url):
    from copy import deepcopy

    from exposed.importer import ImportFailed

    import_members(database_url, ParliamentFixture(1))
    source = declaration()
    source["unknownFutureField"] = True
    other = deepcopy(source)
    other["unknownFutureField"] = 1
    with pytest.raises(ImportFailed, match="Conflicting duplicate"):
        run(database_url, DeclarationsFixture(source, other))
    assert dataset(database_url) == {"declarations": [], "funding": []}


def test_parent_without_required_payer_rejects_child_update(database_url, caplog):
    from tests.declaration_fakes import field, money

    import_members(database_url, ParliamentFixture(1))
    child = declaration(fields=[money("340")])
    child["parentInterestId"] = 500
    parent = declaration(500, fields=[field("PayerName", "Publisher")])
    fixture = DeclarationsFixture(child, parent)
    run(database_url, fixture)
    before = dataset(database_url)
    parent["versions"][0]["fields"][0]["value"] = None
    run(database_url, fixture)
    after = dataset(database_url)
    assert after["declarations"][0] == before["declarations"][0]
    assert after["funding"] == before["funding"]
    assert "Rejected declaration 101" in caplog.text
    assert "payer" in caplog.text


def test_withheld_ultimate_payer_does_not_get_replaced_by_parent_name(database_url):
    from tests.declaration_fakes import field, money

    import_members(database_url, ParliamentFixture(1))
    child = declaration(
        fields=[
            money("340"),
            field("IsUltimatePayerDifferent", True, "Boolean"),
            field("UltimatePayerName", None),
            field("UltimatePayerProtectedByConfidentiality", True, "Boolean"),
        ]
    )
    child["parentInterestId"] = 500
    parent = declaration(500, fields=[field("PayerName", "Intermediary publisher")])
    run(database_url, DeclarationsFixture(child, parent))
    assert dataset(database_url)["funding"][0]["funder"] is None
