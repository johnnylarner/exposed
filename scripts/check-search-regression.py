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
search_results = {}

for case in cases:
    if "search" in case:
        matches = [entity for entity in search_results[case["search"]]
                   if entity["kind"] == "Funder" and entity["name"] == case["expected"]["name"]]
        if len(matches) != 1:
            failed = True
            print(f"FAIL {case['name']}: expected one canonical funder, found {len(matches)}")
            continue
        path = f"/funders/{matches[0]['id']}"
    else:
        path = case["path"]
    url = args.base_url.rstrip("/") + path
    if case["params"]:
        url += "?" + urlencode(case["params"])
    prefix = args.evidence_dir / case["name"]
    prefix.with_suffix(".request.txt").write_text("GET " + url + "\n")
    with urlopen(url, timeout=10) as response:
        body = response.read()
        prefix.with_suffix(".response.json").write_bytes(body)
        actual = json.loads(body)[case["field"]]
    if case["field"] == "entities":
        search_results[case["name"]] = actual
        actual = [{key: value for key, value in entity.items() if key != "id"}
                  for entity in actual]
    else:
        actual = {key: value for key, value in actual.items() if key != "id"}
        member_labels = {member_id: f"member-{index + 1}"
                         for index, member_id in enumerate(actual["member_ids"])}
        actual["member_ids"] = list(member_labels.values())
        for evidence in ["alias_links", "family_links"]:
            actual[evidence] = [
                {**link, "left_funder_id": member_labels[link["left_funder_id"]],
                 "right_funder_id": member_labels[link["right_funder_id"]]}
                for link in actual[evidence]
            ]
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
