# Parliament API client

This package fetches Parliament responses and submits them to the Rust application.
It has no database driver, domain models, funding parser or refresh policy.
Source-specific interpretation and business rules live in Rust; see the
[architecture](../docs/architecture.md).

After [setup](../README.md), start the Rust app and run from the repository root:

```sh
make initialize           # members, then declarations; safe to rerun
make import-members       # manual member/profile/service refresh
make import-declarations  # optional manual declaration refresh
```

From `ingest/`, the equivalent commands are
`.venv/bin/python -m exposed initialize`, `import-members`, and `import-declarations`.
Declarations also refresh daily while the Rust application's timer is enabled.
Members are never written automatically. New MP IDs found during a daily check
appear in Rust's refresh status so an operator can trigger a member refresh.

| Setting | Purpose |
| --- | --- |
| `EXPOSED_APP_URL` / `--app-url` | Rust operator API, default `http://127.0.0.1:7000` |
| `EXPOSED_IMPORT_TOKEN` | Bearer token when the Rust operator listener requires it |
| `PARLIAMENT_TERM_START` / `--term-start` | Optional election date, which must match Rust's configured term |

Operator commands load only the working directory's `.env`, with existing
environment variables taking precedence. `DATABASE_URL` in the example file is
for SQLx migration tooling only; Python never reads it or connects to PostgreSQL.
Rust fixes the observation date once per run in Europe/London.

Stdout contains a JSON summary, stderr contains diagnostics. Exit codes are
0 for completion (including declaration rejections), 1 for import failure,
2 for invalid command configuration, and 130 for interruption. Rust logs per-batch
commits and rejected field paths. A failure retains previously completed batches
or MPs, and a rerun safely reconciles them. Initialization does not run migrations,
delete records omitted by Parliament, or reconstruct lost database IDs.

## Worker protocol

Rust invokes `python -m exposed source` as a persistent worker. It accepts one
JSON object per stdin line and flushes one JSON response per stdout line:

```json
{"operation":"declarations","member":7,"offset":0}
```

Responses contain `payload` (unaltered JSON), `fetched_at` (UTC ISO timestamp), and
`context`. Operations are `current-members`, `member-candidates` (term_start,
as_of, offset), `member-histories` (ids), `declarations` (member, offset), and
`parent` (member, parent). Pages request 100 records. Rust's source adapter decodes
the pagination envelope and asks for the next page after the use case permits it.
API request failures return `error` and exit 1. The worker does not load `.env`.

HTTP requests are sequential with a 0.2-second delay, request/connect timeouts,
and four bounded attempts for transport failures, HTTP 429 and 5xx. Retry-After
is capped at 30 seconds. An unsuccessful source request fails the current Rust
import; run-level backoff belongs to Rust.

`make test-unit`, `make lint` and `make typecheck` verify this package without
PostgreSQL. `make check` also builds Rust and runs its isolated database tests.
