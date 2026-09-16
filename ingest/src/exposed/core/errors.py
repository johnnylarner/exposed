"""Import failures and validation diagnostics safe for command output."""

from pydantic import ValidationError


class ImportValidationError(ValueError):
    """The source is incomplete, inconsistent, or unsuitable for this import."""


def validation_error_message(error: ValidationError) -> str:
    """Report model and field paths without including raw input values."""
    details = []
    for issue in error.errors(include_url=False, include_context=False, include_input=False):
        path = ".".join(str(part) for part in issue["loc"]) or "response"
        details.append(f"{path}: {issue['msg']}")
    return f"Invalid {error.title}: {'; '.join(details)}"


class SourceError(RuntimeError):
    """A required source operation failed; the message is safe for public output."""


class StorageError(RuntimeError):
    """An atomic refresh failed in storage; diagnostic details remain in the cause."""


class ImportFailed(RuntimeError):
    def __init__(self, message: str, *, interrupted: bool = False):
        super().__init__(message)
        self.interrupted = interrupted


def safe_error(exc: BaseException) -> str:
    if isinstance(exc, (ImportValidationError, SourceError, StorageError)):
        return str(exc)
    if isinstance(exc, ValidationError):
        return validation_error_message(exc)
    if isinstance(exc, KeyboardInterrupt):
        return "Import interrupted"
    return f"Unexpected importer failure ({type(exc).__name__})"


class DeclarationParseError(ValueError):
    """One declaration cannot be completely interpreted; log it and retain previous data."""

    def __init__(self, path: str, value: object = None, reason: str | None = None):
        # Adapters can supply an already formatted multi-field diagnostic.
        super().__init__(path if reason is None else f"{path}: {reason}; input={value!r}")


class ParentRequired(Exception):
    """A draft needs its parent's complete interpretation before acceptance."""

    def __init__(self, source_id: int):
        self.source_id = source_id
