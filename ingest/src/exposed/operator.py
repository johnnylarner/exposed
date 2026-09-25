"""Submit source evidence and wait for Rust's next request or committed result."""

import logging
import time
from typing import Any
from uuid import uuid4

import httpx

from exposed.source import SourceClient


class OperatorError(Exception):
    """The Rust application could not complete the import."""


def _post(app: httpx.Client, path: str, body: dict[str, Any]) -> dict[str, Any]:
    # Same session/sequence/body on every retry: a lost acknowledgement must not
    # publish a batch twice or start a second run.
    for attempt in range(3):
        try:
            response = app.post(path, json=body)
        except httpx.TransportError:
            if attempt == 2:
                raise OperatorError("Cannot reach the Rust import interface") from None
            time.sleep(2**attempt)
            continue
        try:
            message = response.json()
        except ValueError:
            raise OperatorError(
                f"Rust returned an invalid response (HTTP {response.status_code})"
            ) from None
        if not isinstance(message, dict):
            raise OperatorError("Rust returned an invalid response envelope")
        if not response.is_success or "error" in message:
            raise OperatorError(message.get("error", f"Rust returned HTTP {response.status_code}"))
        return message
    raise OperatorError("Rust request retries exhausted")


def run_import(
    kind: str, term_start: str | None, app: httpx.Client, source: SourceClient
) -> dict[str, Any]:
    path = f"/imports/{uuid4()}"
    try:
        message = _post(app, path, {"kind": kind, "term_start": term_start})
        while "request" in message:
            evidence = source.fetch(message["request"])
            message = _post(
                app, path + "/evidence", {"sequence": message["sequence"], "response": evidence}
            )
        if not isinstance(message.get("result"), dict):
            raise OperatorError("Rust response has no import result")
        return message["result"]
    finally:
        try:
            app.delete(path, timeout=5)
        except httpx.HTTPError:
            logging.warning("Unable to close import session; it will expire automatically")
