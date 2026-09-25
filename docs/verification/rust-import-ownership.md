# Import ownership migration verification

Starting revision: `eef6697d7b0eb47c477abcc0fc05a8a3b4d5e789`.

The original Python suite could not collect `test_funder_upgrade.py` because its
historic `db/upgrades/normalize_funders.sql` dependency had already been removed.
Excluding that unrelated file, 172 unit tests passed. The 14 targeted storage tests
passed, and 81 declaration/parser/storage tests passed while capturing the
independent compatibility corpus. Domain/storage tests now live at the Rust seams;
Python retains API, operator and dependency-boundary tests.

On 2026-09-25 the running development database was inspected and upgraded in place.
Only baseline version `20260915000000` was installed. Neither new import table
existed. A fresh temporary database was created from the edited baseline.

The additive changes, live-schema comparison and checksum update ran in one
transaction. All six domain tables were locked against concurrent writes while
comparing record fingerprints. Columns, types, defaults, nullability, comments,
constraints, indexes, functions, triggers and the extension schema/version matched
the fresh database before SQLx's SHA-384 checksum was updated. Every imported row,
UUID and audit timestamp was unchanged, verified by ordered full-row SHA-256 hashes.
The temporary comparison database was removed afterward.

| Preserved table | Rows |
| --- | ---: |
| parliament_terms | 1 |
| members | 655 |
| member_terms | 656 |
| declarations | 11,059 |
| funders | 4,054 |
| funding_entries | 8,699 |

No live member/declaration refresh, WhatsApp message, or sender-account change was
performed during implementation. Initialization and refresh tests use disposable
databases and substituted Parliament responses.

The unmodified starting revision fails Clippy with 534 diagnostics under its
blanket `clippy::restriction`, `pedantic`, `nursery`, and `cargo` policy. That
pre-existing lint-policy cleanup is outside this refactor. Rust type/build checks
and behavioral tests are checked separately.

Implementation validation: `make check` passed (7 Python tests, 34 Rust tests,
Ruff lint/format, Pyright, Rust formatting and all-target build checks). The Rust
projection test also checks all 92 independent declaration fixtures. The dev
Docker image built successfully; its Python worker started and exited on EOF with
network access disabled. Compose configuration and SQLx migration status checks
passed. No runtime was deployed over the existing running application.
