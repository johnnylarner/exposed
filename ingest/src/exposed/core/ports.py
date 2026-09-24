"""Capabilities required by a Commons refresh; no transport or database types."""

from collections.abc import Iterator, Sequence
from contextlib import AbstractContextManager
from datetime import date
from typing import Literal, Protocol

from exposed.core.models import Member, MemberHistory, MemberProfile, ServicePeriod

type WriteResult = Literal["inserted", "updated", "unchanged"]


class MemberSource(Protocol):
    """Stream source order, retaining duplicates for the refresh to validate.

    Within each profile stream, identical repeated IDs are logged and collapsed;
    conflicting profiles for one ID fail the refresh, including across batches.
    Candidate batches are bounded by the source. History results must contain
    exactly the requested IDs; incomplete or duplicate histories fail the refresh.
    Implementations translate dependency failures into SourceError and malformed
    data into ImportValidationError. No partial result counts as success.
    """

    def current_commons(self) -> Iterator[MemberProfile]: ...

    def commons_candidates(
        self, term_start: date, as_of: date
    ) -> Iterator[tuple[MemberProfile, ...]]: ...

    def member_histories(self, member_ids: set[int]) -> tuple[MemberHistory, ...]: ...


class MemberWriter(Protocol):
    """Reconcile a profile and all its term service within an active batch.

    Return the profile's change status, retain existing identifiers, replace
    corrected service periods, and leave unmentioned members untouched.
    Writes become visible together only after the batch exits successfully.
    """

    def write_member(self, member: Member, periods: Sequence[ServicePeriod]) -> WriteResult: ...


class RefreshStore(Protocol):
    """One atomic scope for the configured term and one batch of member/service writes.

    Validate the stored term with the core's single-term invariant. Preserve a
    stored term end. Commit on successful exit; roll back on any exception,
    including interruption. Completed batches remain committed if a later batch
    or final refresh validation fails. Translate storage failures into StorageError,
    preserving their diagnostic causes. Assume one import/migration at a time,
    as before; this port adds no concurrency control.
    """

    def refresh_batch(self, term_start: date) -> AbstractContextManager[MemberWriter]: ...
