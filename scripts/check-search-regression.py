#!/usr/bin/env python3
import argparse
import difflib
import json
from pathlib import Path
from urllib.parse import urlencode
from urllib.request import urlopen

parser = argparse.ArgumentParser(description="Compare real HTTP results with the accepted search snapshot.")
parser.add_argument("base_url", help="URL of the API started against exposed_search_regression")
parser.add_argument("evidence_dir", type=Path, help="New directory for requests, responses, and diffs")
parser.add_argument("--cases", type=Path, default=Path(__file__).with_name("search-regression.json"))
args = parser.parse_args()
cases = json.loads(args.cases.read_text())
args.evidence_dir.mkdir(parents=True, exist_ok=False)
failed = False

for case in cases:
    url = args.base_url.rstrip("/") + case["path"]
    if case["params"]:
        url += "?" + urlencode(case["params"])
    prefix = args.evidence_dir / case["name"]
    prefix.with_suffix(".request.txt").write_text("GET " + url + "\n")
    with urlopen(url, timeout=10) as response:
        body = response.read()
        prefix.with_suffix(".response.json").write_bytes(body)
        actual = json.loads(body)[case["field"]]
    expected_text = json.dumps(case["expected"], indent=2, sort_keys=True) + "\n"
    actual_text = json.dumps(actual, indent=2, sort_keys=True) + "\n"
    difference = "".join(difflib.unified_diff(
        expected_text.splitlines(keepends=True), actual_text.splitlines(keepends=True),
        fromfile="expected", tofile="actual",
    ))
    prefix.with_suffix(".expected.json").write_text(expected_text)
    prefix.with_suffix(".diff.txt").write_text(difference)
    if actual != case["expected"]:
        failed = True
        print(f"FAIL {case['name']}\n{difference}")
    else:
        print(f"PASS {case['name']}")

raise SystemExit(1 if failed else 0)
