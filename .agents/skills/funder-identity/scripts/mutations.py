import json
from pathlib import Path
import shutil
import tempfile

from check import ROOT, native, source_record, verify


MUTATIONS = [
    ("company-conflict", "identity.rs", "anchors.entry(number)", "anchors.entry(String::new())",
     "identity::all_small_evidence_graphs_preserve_company_and_type_boundaries", "incompatible identities merged"),
    ("kind-conflict", "identity.rs", "&& kinds.contains(&EntityKind::Organisation)", "&& false",
     "identity::all_small_evidence_graphs_preserve_company_and_type_boundaries", "incompatible identities merged"),
    ("ambiguous-company", "identity.rs", "if ambiguous_attachment {", "if false && ambiguous_attachment {",
     "identity::every_four_node_bridge_abstains_when_both_companies_are_reachable", "competing company attachments must abstain"),
    ("identifier-precedence", "identity.rs", "let number = ids.iter().find_map(|id| company(&input.observations[id]));", "let number: Option<String> = None;",
     "identity::company_identity_precedes_missing_or_present_names", "accepted company identifier precedes name fallback"),
    ("parent-member", "attribution.rs", "observation.member_id == payment.member_id", "true",
     "attribution::every_parent_candidate_and_flag_combination", "parent selection requires false flag and same-member immediate-parent root payer"),
    ("ultimate-precedence", "attribution.rs", "if let Some(id) = &payment.ultimate_payer_funder_id {", "if let Some(id) = &None::<FunderObservationId> {",
     "attribution::every_explicit_ultimate_and_flag_combination", "explicit ultimate payer wins and withheld ultimate never falls back"),
    ("withheld-donor", "attribution.rs", "if usable(input, id) {", "if true {",
     "attribution::every_missing_or_withheld_donor_combination", "absent donor permits payer but withheld donor blocks payer fallback"),
    ("parent-cycle", "attribution.rs", "if parent_cycle(input, payment) {", "if false && parent_cycle(input, payment) {",
     "attribution::every_parent_cycle_ambiguity_and_withholding_combination", "parent cycles outrank ambiguity and unusable parents never fall back to donor"),
    ("nested-metadata", "declaration_source.rs", "entries.push(parse_funding_entry(nested, true)?);", "entries.push(parse_funding_entry(fields, true)?);",
     "source_hierarchy::nested_type_and_identifier_fields_remain_in_their_own_scope", "nested type stays local and missing type is not inherited"),
]

KANI_MUTATIONS = [
    ("company-metadata", "if observation\n        .donor_kind", "if false && observation\n        .donor_kind",
     "identity::person_kind_excludes_company_identifier"),
    ("company-length", "number.len() == 8", "true",
     "identity::company_identifier_rejects_short_value"),
    ("company-characters", "number.bytes().all(|b| b.is_ascii_alphanumeric())", "true",
     "identity::company_identifier_rejects_punctuation"),
    ("company-digit", "number.bytes().any(|b| b.is_ascii_digit())", "true",
     "identity::company_identifier_requires_a_digit"),
    ("company-normalization", ".to_ascii_uppercase()", ".to_owned()",
     "identity::company_identifier_normalizes_case_and_space"),
]


def check_mutations(evidence, timeout):
    evidence.mkdir(parents=True, exist_ok=False)
    before = source_record(ROOT)
    results = []
    with tempfile.TemporaryDirectory(prefix="exposed-identity-mutations-") as temporary:
        source_root = Path(temporary)
        models = Path("exposed/src/lib/domain/models")
        shutil.copytree(ROOT / models, source_root / models)
        source_file = Path("exposed/src/lib/outbound/declaration_source.rs")
        (source_root / source_file).parent.mkdir(parents=True)
        shutil.copy2(ROOT / source_file, source_root / source_file)
        resolution = source_root / models / "declaration_resolution"
        for name, filename, original, replacement, test, expected in MUTATIONS:
            target = source_root / source_file if filename == "declaration_source.rs" else resolution / filename
            clean = target.read_text()
            if clean.count(original) != 1:
                raise RuntimeError(f"{name}: expected one source match, found {clean.count(original)}")
            target.write_text(clean.replace(original, replacement))
            try:
                _, result = native(evidence / name, source_root, test)
                caught = (result.returncode != 0 and "test result: FAILED" in result.stdout
                          and "panicked at" in result.stdout and expected in result.stdout)
                (evidence / name / filename).write_text(target.read_text())
                results.append({"mutation": name, "caught": caught, "expected_assertion": expected})
            finally:
                target.write_text(clean)
            (evidence / "results.json").write_text(json.dumps(results, indent=2) + "\n")

        target = resolution / "identity.rs"
        for name, original, replacement, harness in KANI_MUTATIONS:
            clean = target.read_text()
            if clean.count(original) != 1:
                raise RuntimeError(f"{name}: expected one source match, found {clean.count(original)}")
            target.write_text(clean.replace(original, replacement))
            try:
                _, report = verify(evidence / name, [harness], source_root, timeout)
                checks = [check for result in report.get("verification_results", {}).get("results", []) for check in result.get("checks", [])]
                caught = any(check.get("status") == "Failure" and check.get("category") == "assertion"
                             and "captured company metadata follows type and identifier precedence" in check.get("description", "") for check in checks)
                results.append({"mutation": name, "caught": caught})
                (evidence / name / "identity.rs").write_text(target.read_text())
            finally:
                target.write_text(clean)
            (evidence / "results.json").write_text(json.dumps(results, indent=2) + "\n")
    if source_record(ROOT) != before:
        raise RuntimeError("original production sources changed during mutation validation")
    (evidence / "results.json").write_text(json.dumps(results, indent=2) + "\n")
    return all(result["caught"] for result in results)
