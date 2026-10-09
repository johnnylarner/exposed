import hashlib
import json
from pathlib import Path
import sys

import duckdb

directory = Path(sys.argv[1])
connection = duckdb.connect(":memory:")
fields = {
    "observations": ("funders.parquet", "*"),
    "payments": ("funding_entries.parquet", "funding_entry_id,member_id,parliament_member_id,declaration_id,register_id,funding_ordinal,parent_declaration_id,is_ultimate_payer_different,donor_funder_id,payer_funder_id,ultimate_payer_funder_id"),
}
result = {"input_sha256": {}}
for key, (filename, columns) in fields.items():
    path = directory / filename
    connection.read_parquet(str(path)).create_view(key)
    result[key] = [json.loads(row[0]) for row in connection.execute(f"SELECT to_json(t) FROM (SELECT {columns} FROM {key}) t").fetchall()]
    result["input_sha256"][filename] = hashlib.sha256(path.read_bytes()).hexdigest()
Path(sys.argv[2]).write_text(json.dumps(result, ensure_ascii=False) + "\n")
print({key:len(result[key]) for key in fields})
