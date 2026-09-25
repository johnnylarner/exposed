# Rust import ownership

Agreed in the design discussion, 2026-09-25. This supersedes the Python writer
boundary in the earlier backend and declaration specifications.

Python is a Parliament API client: HTTP, bounded retries, throttling, and raw
response delivery. It must have no database access or domain interpretation.
Rust owns member/service rules, declaration interpretation, funder attribution,
validation, persistence, refresh policy, and notifications. Source-specific
decoding belongs in Rust adapters, with business decisions in the core.

Members are refreshed manually with a Python command talking to the running Rust
application. Rust acknowledges each committed batch before the next source page
is fetched. Declarations refresh automatically in the Rust application, using a
Python CLI adapter. A background task checks persisted refresh state, catches up
after downtime, and retries operational failures with bounded backoff. A complete
source check is due daily; failures must not advance its successful timestamp.
Freshness is relative to Parliament publication, not the date of a real-world event.
Daily checks also report new MPs that require a manual member refresh.

Initialization reuses member refresh followed by declaration refresh. It is safe
to repeat after interruption; it does not apply migrations or promise exact
restoration of lost identifiers. All imports share one concurrency guard.

Preserve: one atomic member batch and one atomic declaration publication per MP;
completed units survive later failures; no inferred deletion from source omissions;
stable entity IDs and unchanged funding IDs (multiset equality); per-declaration
rejections retain previous rows while accepted siblings proceed; raw duplicate
comparison before interpretation; latest-version and parent-payer rules; full
reparsing on every refresh; one configured Parliament with explicit rollover.
Completed traversal with parse rejections is success with warnings, not a fatal
command error. CLI summaries use JSON stdout; diagnostics use stderr. Exit codes
remain 0/1/2/130 for success/failure/configuration/interruption.

Notifications use a core-owned port and a WhatsApp Business Platform adapter.
Delivery failures retry independently of imports. Sender credentials and an
approved template are operator configuration. Notification frequency is deferred
and must remain an application policy; do not start sending during implementation.
Delta ingestion, richer orchestration metadata, rollover, and agent control are
outside this change.

Verification seams agreed during design: import use cases with source/storage
substitution; real atomic persistence; Python API/CLI protocol. Existing Python
characterizations are the compatibility reference while their responsibilities
move to Rust.
