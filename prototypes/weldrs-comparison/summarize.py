import json
from pathlib import Path
import sys

summaries = []
for argument in sys.argv[1:]:
    report = json.loads(Path(argument).read_text())
    cases = report["cases"]
    variants = []
    for name in ("Stock", "NullNeutral", "SplinkCompatible"):
        rows = [next(v for v in case["variants"] if v["variant"] == name) for case in cases]
        differences = [row["comparison"] for row in rows]
        variants.append({
            "variant": name,
            "score_pairs": sum(d["splink_pairs"] for d in differences),
            "candidate_differences": sum(len(d["missing_candidates"]) + len(d["extra_candidates"]) for d in differences),
            "level_differences": sum(len(d["level_differences"]) for d in differences),
            "score_differences": sum(len(d["score_differences"]) for d in differences),
            "maximum_absolute_probability_delta": max(d["maximum_absolute_probability_delta"] for d in differences),
            "policy_cases": sum(row["policy"] is not None for row in rows),
            "policy_differences": sum(row["policy"] is not None and not row["policy"]["equal"] for row in rows),
            "direct_vs_lazy_differences": sum(len(row["direct_vs_lazy"]["score_differences"]) + len(row["direct_vs_lazy"]["level_differences"]) + len(row["direct_vs_lazy"]["missing_candidates"]) + len(row["direct_vs_lazy"]["extra_candidates"]) for row in rows),
            "reversed_input_differences": sum(len(row["reversed_input"]["score_differences"]) + len(row["reversed_input"]["level_differences"]) + len(row["reversed_input"]["missing_candidates"]) + len(row["reversed_input"]["extra_candidates"]) for row in rows),
        })
    summaries.append({"distance_engine":report["distance_engine"], "cases":len(cases), "variants":variants,
        "local_capture":next((c for c in cases if c["case"] == "local_capture"), None)})
for summary in summaries:
    if summary["local_capture"]:
        summary["local_capture"] = {
            "variants": [{"variant":v["variant"], "score_pairs":v["comparison"]["splink_pairs"],
                "score_differences":len(v["comparison"]["score_differences"]),
                "level_differences":len(v["comparison"]["level_differences"]),
                "maximum_absolute_probability_delta":v["comparison"]["maximum_absolute_probability_delta"],
                "policy":v["policy"]} for v in summary["local_capture"]["variants"]]
        }
print(json.dumps(summaries,indent=2))
