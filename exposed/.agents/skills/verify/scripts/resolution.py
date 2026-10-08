#!/usr/bin/env python3
"""Drive the real clean → resolve CLI path with inspectable identity evidence."""

import argparse
from datetime import date, datetime, timezone
import hashlib
import json
from pathlib import Path
import shutil
import sys
import traceback

import pyarrow as pa
import pyarrow.parquet as pq

sys.dont_write_bytecode = True
from declarations import KEY, MEMBER, raw_schema, run_cli, save_json


CASES = [
    (1, "Gary Lubner", "Individual", None, None, None, None),
    (2, "Gary Lubner", "Individual", None, None, None, None),
    (3, "Gary Lubner", "Individual", "20 Other Road", None, None, None),
    (4, "Gary Lubner", "Individual", "30 Other Road", None, None, None),
    (5, "Example Charity", "Charity", "10 Example Road", None, None, None),
    (6, "Example Charity", "Charity", "10 Example Road", None, None, None),
    (7, "Example Ltd", "Company", "1 First Road", "00001234", None, None),
    (8, "Example Ltd", "Company", "2 Second Road", "00001234", None, None),
    (9, "Example Ltd", "Company", "3 Third Road", "00005678", None, None),
    (10, "Alice", "Individual", None, None, None, "Grant Trust"),
    (11, "Other Person", "Individual", None, None, "Grant Trust", None),
]


def fixture(scratch):
    rows = []
    for declaration_id, donor, kind, address, company, payer, ultimate in CASES:
        fields = [
            {"name": "DonorName", "value": donor},
            {"name": "DonorStatus", "value": kind},
            {"name": "Value", "value": "100"},
        ]
        for field, value in [
            ("DonorPublicAddress", address),
            ("DonorCompanyIdentifier", company),
            ("PayerName", payer),
            ("UltimatePayerName", ultimate),
        ]:
            if value is not None:
                fields.append({"name": field, "value": value})
        source = {
            "id": declaration_id,
            "category": {"id": 3, "name": "Donations"},
            "versions": [{"register": {"id": 820, "publishedDate": "2026-09-07"}, "fields": fields}],
        }
        rows.append({
            "member_id": MEMBER,
            "parliament_member_id": 4613,
            "declaration_id": declaration_id,
            "category_id": 3,
            "category_name": "Donations",
            "register_id": 820,
            "register_published_date": date(2026, 9, 7),
            "fetched_at": datetime(2026, 10, 7, tzinfo=timezone.utc),
            "source_json": json.dumps(source),
            "donor_name": donor,
            "payer_name": payer,
            "ultimate_payer_name": ultimate,
            "amount": "100",
            "funder_kind": kind,
            "company_number": company,
        })
    raw = scratch / "data" / KEY / "raw/declarations" / f"{MEMBER}.parquet"
    raw.parent.mkdir(parents=True)
    pq.write_table(pa.Table.from_pylist(rows, schema=raw_schema()), raw)
    (scratch / ".env").write_text("")
    return raw


def verify(entries, funders, observations, attributions, pairs, manifest):
    assert len(entries) == len(CASES), "Funding occurrences changed"
    assert len(funders) == len(CASES) + 2, "Role observations changed"
    by_source = {(row["declaration_id"], row["role"]): row for row in funders}
    assert len(by_source) == len(funders), "Role observations were collapsed"
    by_id = {row["funder_id"]: row for row in funders}
    assignments = {row["funder_id"]: row for row in observations}
    assert len(assignments) == len(funders) and set(assignments) == set(by_id)

    def assigned(declaration_id, role="donor"):
        return assignments[by_source[declaration_id, role]["funder_id"]]

    gary = [assigned(index) for index in range(1, 5)]
    assert len({row["identity_id"] for row in gary}) == 1
    assert {row["identity_basis"] for row in gary} == {"donor_name_link"}
    charity = [assigned(index) for index in (5, 6)]
    assert charity[0]["identity_id"] == charity[1]["identity_id"]
    assert {row["identity_basis"] for row in charity} == {"statistical_link"}
    companies = [assigned(index) for index in (7, 8, 9)]
    assert companies[0]["identity_id"] == companies[1]["identity_id"]
    assert companies[0]["identity_id"] != companies[2]["identity_id"]
    assert {row["identity_basis"] for row in companies} == {"source_reported_company"}
    assert assigned(10, "ultimate_payer")["identity_id"] != assigned(11, "payer")["identity_id"]

    by_entry = {row["funding_entry_id"]: row for row in entries}
    decisions = {row["funding_entry_id"]: row for row in attributions}
    assert len(by_entry) == len(decisions) == len(CASES)
    assert set(by_entry) == set(decisions)
    by_declaration = {row["declaration_id"]: decisions[row["funding_entry_id"]] for row in entries}
    for declaration_id in range(1, 12):
        decision = by_declaration[declaration_id]
        role = "ultimate_payer" if declaration_id == 10 else "donor"
        assert decision["attribution_status"] == "selected"
        assert decision["attribution_basis"] == ("explicit_ultimate_payer" if declaration_id == 10 else "donor")
        assert decision["selected_funder_id"] == by_source[declaration_id, role]["funder_id"]

    gary_ids = {by_source[index, "donor"]["funder_id"] for index in range(1, 5)}
    charity_ids = {by_source[index, "donor"]["funder_id"] for index in (5, 6)}
    gary_pairs = [row for row in pairs if {row["left_funder_id"], row["right_funder_id"]} <= gary_ids]
    charity_pairs = [row for row in pairs if {row["left_funder_id"], row["right_funder_id"]} == charity_ids]
    assert len(gary_pairs) == 6 and len(charity_pairs) == 1
    assert all(row["disposition"] == "accepted" and row["reason"] == "exact_donor_name" and 0 < row["probability"] < 0.999 for row in gary_pairs)
    assert charity_pairs[0]["disposition"] == "accepted"
    assert charity_pairs[0]["reason"] == "exact_name_full_address_threshold"
    assert charity_pairs[0]["probability"] >= 0.999
    assert manifest["policy_version"] == "funder-resolution-v2"
    assert manifest["model"]["splink_version"] == "4.0.17"
    assert manifest["model"]["duckdb_version"] == "1.4.4"
    assert manifest["model"]["calibrated"] is False
    return {"payments": len(entries), "observations": len(funders), "pairs": len(pairs)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["doctor", "drive"])
    parser.add_argument("evidence", type=Path)
    args = parser.parse_args()
    evidence = args.evidence.resolve(strict=True)
    binary = Path((evidence / "binary.txt").read_text().strip())
    checksum = hashlib.sha256(binary.read_bytes()).hexdigest()
    assert checksum == (evidence / "build.sha256").read_text().split()[0], "CLI changed since Launch; rebuild"
    output = evidence / "resolution"
    output.mkdir(exist_ok=True)
    help_result = run_cli(binary, evidence, output, "doctor", ["data", "declarations", "resolve", "--help"])
    assert help_result.returncode == 0 and "--ingestion-key" in help_result.stdout, "Wrong CLI build"
    worker = Path(__file__).resolve().parents[5] / "resolution/worker.py"
    assert worker.is_file()
    import splink
    import duckdb
    assert splink.__version__ == "4.0.17" and duckdb.__version__ == "1.4.4"
    if args.action == "doctor":
        print(f"Resolution Doctor passed: {binary} ({checksum})")
        return
    scratch = output / "scratch"
    scratch.mkdir()
    try:
        raw = fixture(scratch)
        shutil.copy2(raw, output / "raw-input.parquet")
        save_json(output / "cases.json", CASES)
        (scratch / "clean.yaml").write_text("data_dir: ./data\n")
        (scratch / "resolve.yaml").write_text(
            f"data_dir: ./data\nresolution_python: {json.dumps(str(evidence / 'venv/bin/python'))}\n"
            f"resolution_worker: {json.dumps(str(worker))}\ncandidate_budget: 100\n"
        )
        clean = ["data", "declarations", "clean", "clean.yaml", "--ingestion-key", KEY]
        resolve = ["data", "declarations", "resolve", "resolve.yaml", "--ingestion-key", KEY]
        result = run_cli(binary, scratch, output, "clean", clean)
        assert result.returncode == 0, result.stderr
        assert raw.read_bytes() == (output / "raw-input.parquet").read_bytes(), "Cleaning changed capture"
        cleaned = scratch / "data" / KEY / "cleaned/declarations"
        shutil.copytree(cleaned, output / "cleaned")
        shutil.rmtree(raw.parent.parent)
        result = run_cli(binary, scratch, output, "resolve", resolve)
        assert result.returncode == 0, result.stderr
        resolved = scratch / "data" / KEY / "resolved/declarations"
        shutil.copytree(resolved, output / "resolved")
        for name in ("funders.parquet", "funding_entries.parquet"):
            assert (cleaned / name).read_bytes() == (output / "cleaned" / name).read_bytes()
        tables = {name: pq.read_table(resolved / f"{name}.parquet").to_pylist() for name in (
            "observation_resolution", "payment_attribution", "pair_decisions"
        )}
        for name, rows in tables.items():
            save_json(output / f"{name}.json", rows)
        manifest = json.loads((resolved / "manifest.json").read_text())
        counts = verify(
            pq.read_table(cleaned / "funding_entries.parquet").to_pylist(),
            pq.read_table(cleaned / "funders.parquet").to_pylist(),
            tables["observation_resolution"], tables["payment_attribution"],
            tables["pair_decisions"], manifest,
        )
        repeat = run_cli(binary, scratch, output, "repeat", resolve)
        assert repeat.returncode != 0 and "already exist" in repeat.stderr, "Existing resolution was accepted"
        for path in (output / "resolved").iterdir():
            assert path.read_bytes() == (resolved / path.name).read_bytes(), f"Repeat changed {path.name}"
        save_json(output / "proof.json", {
            "features": ["identity-resolution"], **counts, "binary_sha256": checksum,
            "raw_removed_before_resolution": True, "cleaned_unchanged": True,
            "repeat_rejected_without_changes": True,
        })
        print(f"Verified {counts['payments']} payments and {counts['observations']} observations. Evidence: {output}")
    except Exception:
        (output / "failure.txt").write_text(traceback.format_exc())
        raise
    finally:
        shutil.rmtree(scratch)
        (output / "drive-cleanup.txt").write_text("Removed this run's scratch directory. Evidence retained.\n")


if __name__ == "__main__":
    main()
