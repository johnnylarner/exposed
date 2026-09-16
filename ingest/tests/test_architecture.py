"""Guard the documented dependency direction for both ingestion cores."""

import ast
import subprocess
import sys
from pathlib import Path

CORE = Path(__file__).parents[1] / "src/exposed/core"


def test_core_depends_only_on_core_python_and_value_libraries():
    violations = []
    for path in CORE.glob("*.py"):
        for node in ast.walk(ast.parse(path.read_text())):
            modules = []
            if isinstance(node, ast.Import):
                modules = [alias.name for alias in node.names]
            elif isinstance(node, ast.ImportFrom):
                modules = [node.module or ""]
            else:
                continue
            for module in modules:
                if (
                    module.startswith("exposed.core.")
                    or module == "exposed.core"
                    or module.split(".")[0] in sys.stdlib_module_names | {"pydantic"}
                ):
                    continue
                violations.append(f"{path.name}:{node.lineno}: {module}")
    assert violations == []


def test_declaration_refresh_runs_when_infrastructure_imports_are_blocked():
    script = """
import builtins
original_import = builtins.__import__
def without_infrastructure(name, *args, **kwargs):
    if name.split(".")[0] in {"httpx", "psycopg", "dotenv"}:
        raise AssertionError("Infrastructure imported: " + name)
    return original_import(name, *args, **kwargs)
builtins.__import__ = without_infrastructure
from datetime import date
from uuid import UUID
from exposed.core.declarations import DeclarationDraft
from exposed.core.refresh_declarations import refresh_declarations
from tests.declaration_memory import MemoryDeclarationSource, MemoryDeclarationStore
term = date(2024, 7, 4)
source = MemoryDeclarationSource()
source.add(DeclarationDraft(id=101, member_source_id=1, category_id=9,
    category_name="Miscellaneous", funding=(), payer=None))
result = refresh_declarations(term, source, MemoryDeclarationStore(term, {1: UUID(int=1)}))
assert result["declarations"] == 1
"""
    result = subprocess.run(
        [sys.executable, "-c", script], cwd=CORE.parents[2], capture_output=True, text=True
    )
    assert result.returncode == 0, result.stderr
