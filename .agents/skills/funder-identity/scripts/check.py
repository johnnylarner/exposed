#!/usr/bin/env python3
"""Run the bounded production-source Kani proofs and retain their evidence."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import re
from datetime import datetime, timezone

sys.dont_write_bytecode = True

KANI_VERSION = "0.68.0"
SKILL = Path(__file__).resolve().parents[1]
ROOT = SKILL.parents[2]
CRATE = SKILL / "kani"
KANI_HARNESSES = {
    "identity::missing_kind_accepts_valid_identifier",
    "identity::company_kind_accepts_valid_identifier",
    "identity::person_kind_excludes_company_identifier",
    "identity::organisation_kind_excludes_company_identifier",
    "identity::unrecognized_kind_excludes_company_identifier",
    "identity::company_identifier_requires_a_digit",
    "identity::company_identifier_rejects_short_value",
    "identity::company_identifier_rejects_long_value",
    "identity::company_identifier_rejects_punctuation",
    "identity::company_identifier_normalizes_case_and_space",
    "identity::every_ascii_suffix_obeys_company_format",
}


def native(evidence, source_root=ROOT, test=None):
    evidence.mkdir(parents=True, exist_ok=False)
    before = source_record(source_root)
    command = ["cargo", "test", "--manifest-path", str(CRATE / "Cargo.toml"), "--lib"]
    if test:
        command.append(test)
    command.extend(["--", "--nocapture"])
    if test:
        command.append("--exact")
    environment = os.environ.copy()
    environment["EXPOSED_SOURCE_ROOT"] = str(source_root)
    result = subprocess.run(command, cwd=ROOT, env=environment, text=True,
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=180)
    (evidence / "output.log").write_text(result.stdout)
    match = re.search(r"test result: ok\. (\d+) passed; 0 failed; 0 ignored", result.stdout)
    success = (result.returncode == 0 and match is not None
               and int(match[1]) == (1 if test else 9)
               and source_record(source_root) == before)
    (evidence / "run.json").write_text(json.dumps({
        "command": command, "source_sha256": before,
        "exit_code": result.returncode, "status": "passed" if success else "failed",
    }, indent=2) + "\n")
    print(result.stdout, end="", flush=True)
    return success, result


def source_record(root):
    models = root / "exposed/src/lib/domain/models"
    files = sorted(models.glob("declaration_resolution/*.rs")) + [
        models / name for name in (
            "declaration_resolution.rs", "declaration_cleaning.rs", "funder_name.rs",
            "declaration_ingestion.rs", "parliament_member.rs", "entity_ingestion.rs",
        )
    ]
    files.append(root / "exposed/src/lib/outbound/declaration_source.rs")
    record = {
        str(path.relative_to(root)): hashlib.sha256(path.read_bytes()).hexdigest()
        for path in files
    }
    proof_files = [CRATE / "build.rs", CRATE / "Cargo.toml", CRATE / "Cargo.lock"]
    proof_files.extend(CRATE.glob("src/*.rs"))
    proof_files.extend((SKILL / "scripts").glob("*.py"))
    record.update({str(path.relative_to(ROOT)): hashlib.sha256(path.read_bytes()).hexdigest()
                   for path in proof_files})
    return record


def verify(evidence, harnesses=(), source_root=ROOT, timeout=180):
    evidence.mkdir(parents=True, exist_ok=False)
    command = [
        "cargo", "kani", "--manifest-path", str(CRATE / "Cargo.toml"),
        "--output-format", "terse", "-Z", "unstable-options",
        "--harness-timeout", f"{timeout}s", "--export-json", str(evidence / "kani.json"),
    ]
    for harness in harnesses:
        command.extend(["--harness", harness, "--exact"])
    command.extend(["--cbmc-args", "--unwind", "12", "--unwindset", "memcmp.0:64"])
    environment = os.environ.copy()
    environment["EXPOSED_SOURCE_ROOT"] = str(source_root)
    record = {
        "command": command, "source_root": str(source_root),
        "source_sha256": source_record(source_root),
        "commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "git_status": subprocess.check_output(["git", "status", "--short"], cwd=ROOT, text=True),
        "status": "incomplete",
    }
    (evidence / "run.json").write_text(json.dumps(record, indent=2) + "\n")
    with (evidence / "output.log").open("w") as output:
        process = subprocess.Popen(command, cwd=ROOT, env=environment, stdout=subprocess.PIPE,
                                   stderr=subprocess.STDOUT, text=True, start_new_session=True)
        try:
            for line in process.stdout:
                output.write(line)
                output.flush()
                print(line, end="", flush=True)
            code = process.wait()
        except BaseException:
            import signal
            os.killpg(process.pid, signal.SIGTERM)
            process.wait()
            raise
    report_path = evidence / "kani.json"
    report = json.loads(report_path.read_text()) if report_path.exists() else {}
    summary = report.get("verification_results", {}).get("summary", {})
    results = report.get("verification_results", {}).get("results", [])
    success = (
        code == 0 and summary.get("status") == "completed"
        and summary.get("executed", 0) > 0
        and summary.get("executed") == summary.get("total_harnesses")
        and summary.get("failed") == 0
        and summary.get("successful") == summary.get("executed")
        and all(result.get("status") == "Success" for result in results)
        and len(results) == summary.get("executed")
    )
    expected = set(harnesses) if harnesses else KANI_HARNESSES
    success = (success and {result["harness_id"] for result in results} == expected
               and source_record(source_root) == record["source_sha256"])
    record.update(exit_code=code, status="passed" if success else "failed")
    (evidence / "run.json").write_text(json.dumps(record, indent=2) + "\n")
    return success, report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--harness", action="append", default=[])
    parser.add_argument("--timeout", type=int, default=180)
    parser.add_argument("--mutations", action="store_true")
    args = parser.parse_args()
    if args.timeout <= 0:
        parser.error("timeout must be positive")
    if args.mutations and args.harness:
        parser.error("mutation validation requires the complete baseline")
    version = subprocess.check_output(["cargo", "kani", "--version"], text=True).strip()
    if version.splitlines()[0] != f"Kani Rust Verifier {KANI_VERSION} (cargo plugin)":
        raise SystemExit(f"Expected Kani {KANI_VERSION}, found {version!r}. See SKILL.md for setup.")
    evidence_root = ROOT / "target/funder-identity-evidence"
    evidence_root.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix=datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ-"), dir=evidence_root))
    print(f"Evidence: {run}", flush=True)
    native_ok, _ = native(run / "native")
    success, _ = verify(run / "kani", args.harness, timeout=args.timeout)
    if args.mutations:
        if not (native_ok and success):
            return 1
        from mutations import check_mutations
        success = check_mutations(run / "mutations", args.timeout)
        restored_native, _ = native(run / "restored-native")
        restored_kani, _ = verify(run / "restored-kani", timeout=args.timeout)
        success = success and restored_native and restored_kani
    (run / "summary.json").write_text(json.dumps({
        "native": native_ok, "kani_and_mutations": success, "complete": native_ok and success,
    }, indent=2) + "\n")
    return 0 if native_ok and success else 1


if __name__ == "__main__":
    sys.exit(main())
