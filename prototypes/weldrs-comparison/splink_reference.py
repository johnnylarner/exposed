import importlib.metadata
import importlib.util
import json
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("reference", root / "resolution/generate.py")
reference = importlib.util.module_from_spec(spec)
spec.loader.exec_module(reference)
model = json.loads((root / "exposed/resources/funder-resolution/model.json").read_text())
for package, version in (("splink", model["splink_version"]), ("duckdb", model["duckdb_version"])):
    if importlib.metadata.version(package) != version:
        raise RuntimeError(f"Expected {package} {version}")
cases = json.loads(Path(sys.argv[1]).read_text())
cases["generator"] = {
    "splink": importlib.metadata.version("splink"),
    "duckdb": importlib.metadata.version("duckdb"),
    "source": "resolution/generate.py original_prediction",
}
for case in cases["cases"]:
    case["pairs"], _ = reference.original_prediction(model, case["rows"])
    print(f"Splink {case['name']}: {len(case['pairs'])} pairs")
Path(sys.argv[2]).write_text(json.dumps(cases, indent=2, ensure_ascii=False) + "\n")
