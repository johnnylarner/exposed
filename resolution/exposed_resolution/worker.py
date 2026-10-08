"""Score feature profiles with a frozen, explicitly uncalibrated Splink model."""
from copy import deepcopy
import importlib.metadata
import json
import math
import platform
import sys

from pathlib import Path

MODEL = json.loads(Path(__file__).with_name("model.json").read_text(encoding="utf-8"))
MODEL_VERSION = MODEL["model_version"]
SPLINK_VERSION = MODEL["splink_version"]
SETTINGS = MODEL["settings"]


def score(request):
    if request.get("version") != 2:
        raise ValueError("unsupported scoring request version")
    budget = request.get("candidate_budget")
    if not isinstance(budget, int) or isinstance(budget, bool) or budget <= 0:
        raise ValueError("candidate_budget must be a positive integer")
    rows = request["rows"]
    keys = set()
    for row in rows:
        key = row["key"]
        if not isinstance(key, str) or not key or key in keys:
            raise ValueError("profile keys must be distinct nonempty strings")
        keys.add(key)
        if not isinstance(row["name"], str) or not row["name"]:
            raise ValueError("profile name must be a usable string")
        if row["address"] is not None and (not isinstance(row["address"], str) or not row["address"]):
            raise ValueError("profile address must be a usable string or null")
        blocking_keys = row.get("blocking_keys")
        if not isinstance(blocking_keys, list) or any(not isinstance(key, str) or not key for key in blocking_keys):
            raise ValueError("blocking_keys must be a list of usable strings")
    if any(not isinstance(row.get("needs_resolution"), bool) for row in rows):
        raise ValueError("needs_resolution must be a boolean")
    actual = importlib.metadata.version("splink")
    if actual != SPLINK_VERSION:
        raise RuntimeError(f"Splink {SPLINK_VERSION} required; found {actual}")
    import duckdb
    if duckdb.__version__ != "1.4.4":
        raise RuntimeError(f"DuckDB 1.4.4 required; found {duckdb.__version__}")
    from splink import DuckDBAPI, Linker

    connection = duckdb.connect(":memory:")
    try:
        connection.execute('CREATE TABLE profiles ("key" VARCHAR, name VARCHAR, address VARCHAR, blocking_keys VARCHAR[], needs_resolution BOOLEAN)')
        if rows:
            connection.executemany("INSERT INTO profiles VALUES (?, ?, ?, ?, ?)", [(r["key"], r["name"], r["address"], r["blocking_keys"], r["needs_resolution"]) for r in rows])
        rules = " OR ".join(f"({rule})" for rule in SETTINGS["blocking_rules_to_generate_predictions"])
        count = connection.execute(f'SELECT count(*) FROM profiles l JOIN profiles r ON l."key" < r."key" AND ({rules})').fetchone()[0]
        if count > budget:
            raise ValueError(f"candidate budget {budget} exceeded by {count} profile pairs; raise candidate_budget explicitly")
        pairs = []
        if count:
            linker = Linker("profiles", deepcopy(SETTINGS), db_api=DuckDBAPI(connection=connection))
            for row in linker.inference.predict(threshold_match_probability=0.0).as_record_dict():
                left, right = sorted([row["key_l"], row["key_r"]])
                probability = float(row["match_probability"])
                if not math.isfinite(probability):
                    raise ValueError("Splink returned a non-finite probability")
                pairs.append({"left": left, "right": right, "probability": probability,
                              "name_level": int(row["gamma_name"]), "address_level": int(row["gamma_address"])})
            if len(pairs) != count:
                raise ValueError("Splink did not return every generated candidate")
        return {"version": 1, "model": {"model_version": MODEL_VERSION, "splink_version": actual,
                "duckdb_version": duckdb.__version__, "python_version": platform.python_version(),
                "calibrated": False, "settings": SETTINGS}, "pairs": sorted(pairs, key=lambda p: (p["left"], p["right"]))}
    finally:
        connection.close()


def run():
    if len(sys.argv) != 3:
        raise ValueError("usage: worker.py INPUT.json OUTPUT.json")
    with open(sys.argv[1], encoding="utf-8") as stream:
        request = json.load(stream)
    result = score(request)
    with open(sys.argv[2], "x", encoding="utf-8") as stream:
        json.dump(result, stream, allow_nan=False, indent=2)


def main():
    try:
        run()
    except Exception as error:
        print(f"Splink scoring failed: {error}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
