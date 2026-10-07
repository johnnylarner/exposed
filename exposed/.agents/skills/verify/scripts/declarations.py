#!/usr/bin/env python3
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import traceback

import pyarrow as pa
import pyarrow.parquet as pq


ROLES = {"donor": "DonorName", "payer": "PayerName", "ultimate_payer": "UltimatePayerName"}
KEY = "00000000-0000-4000-8000-000000000001"
MEMBER = "00000000-0000-4000-8000-000000000002"
SKILL = Path(__file__).resolve().parent.parent


def save_json(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False, default=str) + "\n")


def run_cli(binary, cwd, evidence, label, args):
    command = [str(binary), *args]
    record = {"command": command, "cwd": str(cwd), "DATABASE_URL": "unset"}
    save_json(evidence / f"{label}.command.json", record)
    env = {key: value for key, value in os.environ.items() if key != "DATABASE_URL"}
    result = subprocess.run(command, cwd=cwd, env=env, capture_output=True, text=True, timeout=60)
    (evidence / f"{label}.stdout.txt").write_text(result.stdout)
    (evidence / f"{label}.stderr.txt").write_text(result.stderr)
    record["exit_code"] = result.returncode
    save_json(evidence / f"{label}.command.json", record)
    return result


def fixtures(directory):
    from datetime import date, datetime, timezone

    schema = pa.schema([
        pa.field("member_id", pa.string(), False),
        *[pa.field(name, pa.uint32(), nullable) for name, nullable in [
            ("parliament_member_id", False), ("declaration_id", False),
            ("parent_declaration_id", True), ("category_id", False),
        ]],
        pa.field("category_name", pa.string(), False),
        pa.field("register_id", pa.uint32(), False),
        pa.field("register_published_date", pa.date32(), False),
        pa.field("registration_date", pa.date32(), True),
        *[pa.field(name, pa.string()) for name in [
            "ultimate_payer_name", "donor_name", "payer_name", "amount", "currency",
            "payment_type", "funder_kind", "company_number",
        ]],
        pa.field("is_ultimate_payer_different", pa.bool_()),
        pa.field("fetched_at", pa.timestamp("us", tz="+00:00"), False),
        pa.field("source_json", pa.string(), False),
    ])
    cases = json.loads((SKILL / "name-cases.json").read_text())
    declarations = [
        (1, [{"donor": "Sir Trevor Chinn", "ultimate_payer": "Labour Together Limited"}] * 2, False),
        (2, [{"donor": "Sir Trevor Chinn", "ultimate_payer": "Sir Trevor Chinn"}], True),
        *[(index + 3, [{role: case["name"] for role in ROLES}], False)
          for index, case in enumerate(cases)],
        (20, [], False),
        (21, [{}], False),
    ]
    rows, expected_entries, expected_funders = [], [], []
    for declaration_id, entries, top_level in declarations:
        groups = []
        for ordinal, names in enumerate(entries):
            fields = [{"name": "Value", "value": "2000.00", "typeInfo": {"currencyCode": "GBP"}},
                      {"name": "DonorStatus", "value": "Company"},
                      {"name": "DonorCompanyIdentifier", "value": "00001234"}]
            fields += [{"name": "Name" if role == "donor" and not top_level else ROLES[role],
                        "value": name} for role, name in names.items()]
            groups.append(fields)
            expected_entries.append((declaration_id, ordinal, names))
            for role, name in (names or {"donor": None}).items():
                expected_funders.append((declaration_id, ordinal, role, name))
        fields = groups[0] if top_level else [{"name": "Donors", "values": groups}]
        if declaration_id == 20:
            fields = [{"name": "PayerName", "value": "Declaration Payer Ltd"}]
            expected_funders.append((20, None, "payer", "Declaration Payer Ltd"))
        source = {"id": declaration_id, "category": {"id": 3, "name": "Donations"},
                  "versions": [{"register": {"id": 820, "publishedDate": "2026-09-07"},
                                "registrationDate": "2026-09-01", "fields": fields}]}
        for names in entries or [{}]:
            row = {"member_id": MEMBER, "parliament_member_id": 4613,
                   "declaration_id": declaration_id, "category_id": 3,
                   "category_name": "Donations", "register_id": 820,
                   "register_published_date": date(2026, 9, 7),
                   "registration_date": date(2026, 9, 1),
                   "fetched_at": datetime(2026, 10, 7, tzinfo=timezone.utc),
                   "source_json": json.dumps(source, ensure_ascii=False)}
            if entries:
                row.update(amount="2000.00", currency="GBP", funder_kind="Company", company_number="00001234")
                row.update({f"{role}_name": name for role, name in names.items()})
            rows.append(row)
    path = directory / "data" / KEY / "raw/declarations" / f"{MEMBER}.parquet"
    path.parent.mkdir(parents=True)
    pq.write_table(pa.Table.from_pylist(rows, schema=schema), path)
    (directory / ".env").write_text("")
    (directory / "config.yaml").write_text("data_dir: ./data\n")
    return path, cases, expected_entries, expected_funders


def verify_tables(entries, funders, cases, expected_entries, expected_funders):
    assert len(entries) == len(expected_entries), "Funding occurrence count changed"
    assert len(funders) == len(expected_funders), "Funder observation count changed"
    by_id = {row["funder_id"]: row for row in funders}
    assert len(by_id) == len(funders), "Funder observations were merged"
    assert len({row["funding_entry_id"] for row in entries}) == len(entries), "Repeated payments were merged"
    by_entry = {(row["declaration_id"], row["funding_ordinal"]): row for row in entries}
    by_observation = {(row["declaration_id"], row["funding_ordinal"], row["role"]): row for row in funders}
    expected_keys = {(decl, ordinal, role) for decl, ordinal, role, _ in expected_funders}
    assert set(by_observation) == expected_keys, "Unexpected role or source scope"
    for decl, ordinal, names in expected_entries:
        entry = by_entry[decl, ordinal]
        assert entry["amount"] == "2000.00" and entry["currency"] == "GBP"
        for role in ROLES:
            assert entry[f"{role}_name"] == names.get(role), "Original funding name changed"
            ref = entry[f"{role}_funder_id"]
            observation = by_observation.get((decl, ordinal, role))
            if observation is None:
                assert ref is None, "Absent role acquired a funder"
            else:
                assert by_id[ref] == observation, "Funding reference points to the wrong role"
                assert observation["funding_entry_id"] == entry["funding_entry_id"]
    for decl, ordinal, role, name in expected_funders:
        observation = by_observation[decl, ordinal, role]
        assert observation["name_raw"] == name, "Original funder name changed"
        assert observation["source_scope"] == ("declaration" if ordinal is None else "funding_entry")
        assert observation["member_id"] == MEMBER and observation["register_id"] == 820
        assert observation["source_pointer"].startswith("/versions/0/fields")
        assert observation["donor_kind"] == ("Company" if role == "donor" else None)
        assert observation["donor_company_number"] == ("00001234" if role == "donor" else None)
    assert by_observation[20, None, "payer"]["funding_entry_id"] is None
    assert by_observation[21, 0, "donor"]["name_status"] == "missing"
    assert by_observation[1, 0, "donor"]["person_core"] == "trevor chinn"
    assert by_observation[1, 0, "ultimate_payer"]["organisation_core"] == "labour together"
    identity = {"funder_id", "member_id", "declaration_id", "register_id", "role", "source_scope",
                "source_pointer", "funding_entry_id", "funding_ordinal", "donor_kind", "donor_company_number"}
    for index, case in enumerate(cases):
        variants = []
        for role in ROLES:
            row = by_observation[index + 3, 0, role]
            for column, expected in {"name_status": "present", **case["expected"]}.items():
                assert row[column] == expected, f"{case['name']!r} {role}.{column}: {row[column]!r} != {expected!r}"
            variants.append({column: value for column, value in row.items() if column not in identity})
        assert variants[0] == variants[1] == variants[2], f"Cleaning differs by role: {case['name']!r}"


def main():
    parser = argparse.ArgumentParser(description="Drive Exposed declaration cleaning and preserve Parquet evidence.")
    parser.add_argument("action", choices=["doctor", "drive"])
    parser.add_argument("evidence", type=Path)
    args = parser.parse_args()
    evidence = args.evidence.resolve(strict=True)
    binary = Path((evidence / "binary.txt").read_text().strip())
    checksum = hashlib.sha256(binary.read_bytes()).hexdigest()
    assert checksum == (evidence / "build.sha256").read_text().split()[0], "CLI changed since Launch; rebuild"
    help_result = run_cli(binary, evidence, evidence, "doctor", ["data", "declarations", "clean", "--help"])
    assert help_result.returncode == 0 and "--ingestion-key" in help_result.stdout, "Wrong CLI build"
    if args.action == "doctor":
        print(f"Doctor passed: {binary} ({checksum})")
        return
    scratch = evidence / "scratch"
    scratch.mkdir()
    try:
        raw, cases, expected_entries, expected_funders = fixtures(scratch)
        shutil.copy2(raw, evidence / "raw-input.parquet")
        shutil.copy2(SKILL / "name-cases.json", evidence / "name-cases.json")
        command = ["data", "declarations", "clean", "config.yaml", "--ingestion-key", KEY]
        result = run_cli(binary, scratch, evidence, "clean", command)
        assert result.returncode == 0, result.stderr
        assert raw.read_bytes() == (evidence / "raw-input.parquet").read_bytes(), "Raw capture changed"
        output = scratch / "data" / KEY / "cleaned/declarations"
        assert {path.name for path in output.iterdir()} == {"funders.parquet", "funding_entries.parquet"}
        shutil.copytree(output, evidence / "tables")
        entries = pq.read_table(output / "funding_entries.parquet").to_pylist()
        funders = pq.read_table(output / "funders.parquet").to_pylist()
        save_json(evidence / "funding_entries.json", entries)
        save_json(evidence / "funders.json", funders)
        verify_tables(entries, funders, cases, expected_entries, expected_funders)
        repeat = run_cli(binary, scratch, evidence, "repeat", command)
        assert repeat.returncode != 0 and "already exist" in repeat.stderr, "Existing output was accepted"
        assert {path.name for path in output.iterdir()} == {"funders.parquet", "funding_entries.parquet"}
        for name in ["funders.parquet", "funding_entries.parquet"]:
            assert (output / name).read_bytes() == (evidence / "tables" / name).read_bytes(), "Existing table changed"
        assert raw.read_bytes() == (evidence / "raw-input.parquet").read_bytes(), "Repeat changed raw capture"
        save_json(evidence / "proof.json", {
            "features": ["funding-roles", "name-features", "capture-preservation"],
            "funding_entries": len(entries), "funder_observations": len(funders),
            "name_cases_across_all_roles": len(cases), "binary_sha256": checksum,
            "raw_unchanged": True, "repeat_rejected_without_changes": True,
        })
        print(f"Verified {len(entries)} funding entries and {len(funders)} funder observations. Evidence: {evidence}")
    except Exception:
        (evidence / "failure.txt").write_text(traceback.format_exc())
        raise
    finally:
        shutil.rmtree(scratch)
        (evidence / "drive-cleanup.txt").write_text("Removed this run's scratch directory. Evidence retained.\n")


if __name__ == "__main__":
    main()
