"""Compatibility entry point; refresh decisions live in the declaration core."""

from exposed.adapters.declarations import parsing_error as parsing_error
from exposed.composition import run_declaration_import as run_import

__all__ = ["run_import", "parsing_error"]
