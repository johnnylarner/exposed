"""The production Python package must remain an API transport."""

import ast
from pathlib import Path


def test_python_has_no_database_or_domain_dependencies():
    root = Path(__file__).parents[1] / "src" / "exposed"
    forbidden = {"psycopg", "sqlalchemy", "pydantic", "exposed.core", "exposed.db"}
    for file in root.rglob("*.py"):
        for node in ast.walk(ast.parse(file.read_text())):
            if isinstance(node, ast.Import):
                modules = [alias.name for alias in node.names]
            elif isinstance(node, ast.ImportFrom):
                modules = [node.module or ""]
            else:
                continue
            assert not any(
                module == name or module.startswith(name + ".")
                for module in modules
                for name in forbidden
            ), file
