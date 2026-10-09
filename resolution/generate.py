"""Regenerate the frozen DuckDB program and independent Splink reference scores."""

import argparse
from copy import deepcopy
import hashlib
import importlib.metadata
import json
from pathlib import Path
import re

import duckdb
from splink import DuckDBAPI, Linker


ROOT = Path(__file__).resolve().parents[1]
BUNDLE = ROOT / "exposed/resources/funder-resolution"
REFERENCE = ROOT / "exposed/tests/fixtures/funder-resolution.json"
SQL_FILES = ("schema.sql", "count.sql", "concat.sql", "candidates.sql", "predict.sql", "output.sql")
STAGES = ("__splink__df_concat_with_tf", "__splink__blocked_id_pairs", "__splink__df_predict")
SCHEMA = '''CREATE TABLE profile_rows (
    "key" VARCHAR PRIMARY KEY,
    name VARCHAR NOT NULL,
    address VARCHAR,
    needs_resolution BOOLEAN NOT NULL
);
CREATE TABLE profile_blocking_keys (
    profile_key VARCHAR NOT NULL,
    ordinal UINTEGER NOT NULL,
    blocking_key VARCHAR NOT NULL
);
CREATE VIEW profiles AS
SELECT p."key", p.name, p.address,
       coalesce(
           (SELECT list(k.blocking_key ORDER BY k.ordinal)
            FROM profile_blocking_keys k WHERE k.profile_key = p."key"),
           []::VARCHAR[]
       ) AS blocking_keys,
       p.needs_resolution
FROM profile_rows p;
'''
OUTPUT = '''SELECT key_l, key_r, match_probability, gamma_name, gamma_address
FROM __splink__df_predict
ORDER BY key_l, key_r;
'''


def row(key, name, address=None, needs_resolution=True, blocking_keys=()):
    return dict(key=key, name=name, address=address,
                needs_resolution=needs_resolution, blocking_keys=list(blocking_keys))


def cases():
    levels = [row(f"{name_index}-{address_index}", name, address, blocking_keys=["all-levels"])
              for name_index, name in enumerate(("john smith", "jon smith", "different organisation"))
              for address_index, address in enumerate(("1 road", "2 road", None))]
    return [
        ("empty", []),
        ("singleton", [row("a", "john smith")]),
        ("comparison_levels", levels),
        ("exact_fuzzy_missing", [row("a", "john smith", "1 road"), row("b", "john smith", "1 road"),
                                  row("c", "jon smith", "1 road"), row("d", "john smith")]),
        ("exact_name", [row("a", "same name"), row("b", "same name")]),
        ("exact_address", [row("a", "alpha", "1 road"), row("b", "zulu", "1 road")]),
        ("three_character_prefix", [row("a", "example charity"), row("b", "example foundation")]),
        ("initial_surname", [row("a", "john smith"), row("b", "jon smith"), row("c", "james smyth")]),
        ("anchored_profiles", [row("a", "acme", "1 road", False), row("b", "acme", "1 road", False),
                               row("c", "acme", "1 road")]),
        ("only_anchors", [row("a", "acme", "1 road", False), row("b", "acme", "1 road", False)]),
        ("trade_union", [row("a", "unite union", blocking_keys=["trade-union:unite"]),
                          row("b", "east midlands unite the union", blocking_keys=["trade-union:unite"]),
                          row("c", "united against hunger")]),
        ("alias", [row("a", "northstar consulting trading as fletchers", blocking_keys=["funder-name:fletchers"]),
                    row("b", "fletchers", blocking_keys=["funder-name:fletchers"])]),
        ("generic_suffix", [row("a", "alpha ltd"), row("b", "amber ltd")]),
        ("suffix_on_left", [row("a", "amy company"), row("b", "ann collins")]),
        ("suffix_on_right", [row("a", "ann collins"), row("b", "amy company")]),
        ("empty_regex_match", [row("a", "alpha!"), row("b", "amber?")]),
        ("short_names", [row("a", "a"), row("b", "ab"), row("c", "ác")]),
        ("unicode", [row("a", "éva smith", "1 road"), row("b", "eva smith", "1 road"),
                      row("c", "伊藤", blocking_keys=["unicode"]), row("d", "伊東", blocking_keys=["unicode"])]),
        ("escaped_keys", [row("a", "alpha", blocking_keys=['quote"', "line\nbreak", "slash\\", "éva", "éva"]),
                           row("b", "zulu", blocking_keys=["line\nbreak", "slash\\", "éva"])]),
        ("escaped_key_collisions", [row("a", "alpha", blocking_keys=["line\nbreak"]),
                                     row("b", "zulu", blocking_keys=["linenbreak"]),
                                     row("c", "omega", blocking_keys=["line\\nbreak"]),
                                     row("d", "beta", blocking_keys=["line\nbreak"])]),
        ("repeated_profile", [row("profile-00000000", "same", "same"),
                               row("profile-00000000-repeat", "same", "same")]),
        ("overlapping_rules", [row(key, "same", "same", blocking_keys=["same"]) for key in ("a", "b", "c")]),
    ]


def count_sql(model):
    rules = " OR ".join(f"({rule})" for rule in model["settings"]["blocking_rules_to_generate_predictions"])
    return f'SELECT count(*) FROM profiles l JOIN profiles r ON l."key" < r."key" AND ({rules});\n'


def original_prediction(model, rows, capture=False):
    statements = []

    class CapturingAPI(DuckDBAPI):
        def _execute_sql_against_backend(self, sql):
            if sql.lstrip().upper().startswith("CREATE TABLE"):
                statements.append(sql)
            return super()._execute_sql_against_backend(sql)

    with duckdb.connect(":memory:") as connection:
        connection.execute('CREATE TABLE profiles ("key" VARCHAR, name VARCHAR, address VARCHAR, blocking_keys VARCHAR[], needs_resolution BOOLEAN)')
        if rows:
            connection.executemany("INSERT INTO profiles VALUES (?, ?, ?, ?, ?)",
                                   [(r["key"], r["name"], r["address"], r["blocking_keys"], r["needs_resolution"]) for r in rows])
        count = connection.execute(count_sql(model)).fetchone()[0]
        pairs = []
        if count:
            linker = Linker("profiles", deepcopy(model["settings"]), db_api=CapturingAPI(connection=connection))
            prediction = linker.inference.predict(threshold_match_probability=0.0)
            pairs = [dict(left=r["key_l"], right=r["key_r"], probability=r["match_probability"],
                          name_level=r["gamma_name"], address_level=r["gamma_address"])
                     for r in prediction.as_record_dict()]
        if len(pairs) != count:
            raise RuntimeError("original Splink candidate cardinality differs from worker count")
    if capture and len(statements) != len(STAGES):
        raise RuntimeError(f"expected three generated stages, found {len(statements)}")
    return sorted(pairs, key=lambda p: (p["left"], p["right"])), statements


def normalize(statements):
    names = {}
    for statement, expected in zip(statements, STAGES, strict=True):
        match = re.match(r"\s*CREATE TABLE\s+(\w+)\s+AS\b", statement, re.IGNORECASE)
        if match is None or not match[1].startswith(expected + "_"):
            raise RuntimeError(f"unexpected generated stage {expected}")
        names[match[1]] = expected
    normalized = []
    for statement in statements:
        for physical, stable in names.items():
            statement = re.sub(rf"\b{re.escape(physical)}\b", stable, statement)
        statement = "\n".join(line.rstrip() for line in statement.strip().splitlines())
        normalized.append(statement.rstrip(";") + ";\n")
    return normalized


def replay(program, rows, expected):
    with duckdb.connect(":memory:") as connection:
        connection.execute(program["schema.sql"])
        for r in rows:
            connection.execute("INSERT INTO profile_rows VALUES (?, ?, ?, ?)",
                               [r["key"], r["name"], r["address"], r["needs_resolution"]])
            for ordinal, key in enumerate(r["blocking_keys"]):
                connection.execute("INSERT INTO profile_blocking_keys VALUES (?, ?, ?)", [r["key"], ordinal, key])
        count = connection.execute(program["count.sql"]).fetchone()[0]
        actual = []
        if count:
            for stage in ("concat.sql", "candidates.sql", "predict.sql"):
                connection.execute(program[stage])
            actual = [dict(left=r[0], right=r[1], probability=r[2], name_level=r[3], address_level=r[4])
                      for r in connection.execute(program["output.sql"]).fetchall()]
        if count != len(expected) or actual != expected:
            raise RuntimeError(f"SQL replay differs from original Splink: {actual!r} != {expected!r}")


def render(model):
    if importlib.metadata.version("splink") != model["splink_version"] or duckdb.__version__ != model["duckdb_version"]:
        raise RuntimeError("install the pinned resolution/requirements.txt generator dependencies")
    _, statements = original_prediction(model, cases()[3][1], capture=True)
    concat, candidates, predict = normalize(statements)
    program = dict(zip(SQL_FILES, (SCHEMA, count_sql(model), concat, candidates, predict, OUTPUT), strict=True))
    fixtures = []
    for name, rows in cases():
        expected, _ = original_prediction(model, rows)
        replay(program, rows, expected)
        fixtures.append(dict(name=name, rows=rows, pairs=expected))
    metadata = dict(format_version=1, model_sha256=hashlib.sha256((BUNDLE / "model.json").read_bytes()).hexdigest(),
                    sql_sha256=hashlib.sha256(b"\0".join(program[f].encode() for f in SQL_FILES)).hexdigest(),
                    generator=dict(splink_version=model["splink_version"], duckdb_version=model["duckdb_version"]))
    files = {BUNDLE / name: content for name, content in program.items()}
    files[BUNDLE / "artifact.json"] = json.dumps(metadata, indent=2, ensure_ascii=False, allow_nan=False) + "\n"
    files[REFERENCE] = json.dumps(dict(generator=metadata["generator"], cases=fixtures), indent=2, ensure_ascii=False, allow_nan=False) + "\n"
    return files


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="refuse if checked-in SQL or reference fixtures differ")
    options = parser.parse_args()
    model = json.loads((BUNDLE / "model.json").read_text())
    files = render(model)
    differences = [str(path.relative_to(ROOT)) for path, content in files.items()
                   if not path.is_file() or path.read_text() != content]
    if options.check and differences:
        raise SystemExit("generated resolution artifacts differ: " + ", ".join(differences))
    if not options.check:
        for path, content in files.items():
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content)
    print(f"{'Checked' if options.check else 'Generated'} frozen SQL and {len(cases())} original-Splink reference cases")


if __name__ == "__main__":
    main()
