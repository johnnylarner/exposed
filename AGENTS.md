# Project summary and structure

The goal of this project is to make declaration data easy to search, browse and interpret.

- `exposed/` contains the rust application and CLI code that powers the project.
- `frontend/` contains a simple Svelte application
- `ingest/` contains the legacy Python scripts used to bootstrap this project.

## Exposed coding policy

- Follow with strict detail to the existing style of the application. This includes:
  - Type driven design
  - Domain modelling and parse-not-validate
  - Consistent error handling
  - Service layer abstractions

- Interactions with the database must use `sqlx` macros for compile-time query verification.

- For Rust changes, read and apply the
  [Rust domain development skill](.agents/skills/rust-domain-development/SKILL.md).
- Before implementation, identify the domain states, invariants, and the type
  that owns each rule. Parse external values into those types at boundaries and
  retain typed values internally.
- Use one canonical domain enum for each finite vocabulary. Do not duplicate
  its variants in string allowlists. Use exhaustive matches where behavior
  depends on a variant.
- When a bug exposes duplicated domain rules, remove the duplication rather
  than synchronizing the copies. Trace changes through producers, serialization,
  consumers, and persistence together.

## Frontend coding policy

- The frontend is largely in a prototype stage so changes can be made freely without much guidance.
- Every PR that changes the UI must include desktop and mobile screenshots in
  its description. Capture the affected views from the running application on
  the PR branch after completing the changes.
- Upload screenshots as GitHub attachments. Never commit PR screenshots to the
  repository. Verify that the attachments render in the PR description before
  marking the PR ready for review.


## Ingest coding policy

- When porting code from this project, do not replicate the style. Use it as an end to end guide for functionality.

## Worktree policy

- For read-only tasks (investigating, reading, summarizing, or describing code),
  work on the current branch without creating a worktree.
- For tasks that require changing code or other files, work in a dedicated Git
  worktree before making changes. If a read-only task expands to require changes,
  switch to a dedicated worktree first.
- For such changes, reuse this chat's dedicated worktree if one already exists.
  Otherwise, create a sibling worktree with a unique branch and perform all
  edits, installs, and checks there.
- Never modify the primary checkout.
- Report the worktree path and branch when starting work in a dedicated worktree.

## Git history

- After completing and validating a task, commit its changes on the task's work
  branch without waiting for a separate request.
- Never commit directly on `main`.
- Keep Git history linear. Rebase feature branches onto the target branch and
  integrate changes with a fast-forward or squash; do not create merge commits.

## Development database migrations

- While the project is in development, keep one reversible baseline version in
  `db/migrations/20260915000000_initial.up.sql` and its `.down.sql` counterpart.
  Fold schema changes into this pair without adding migration versions.
- For an existing database, inspect its applied schema and apply the necessary
  changes in place to preserve imported data. SQLx rejects an edited baseline's
  checksum; verify the live schema before updating its recorded checksum using
  the procedure in `db/README.md` or reporting a migration complete.
