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

## Financial totals contract

- The backend member summary owns declared GBP totals across **all categories**.
  Frontends display its mandatory fields without recalculation or fallback sums.
- Changes to financial meaning require an explicit user request and a PR description
  of effects on included records/categories, currency, registration dates, missing
  values, precision, and labels. UI edits must preserve that meaning.
- Run `scripts/check-declared-totals-contract.sh` for changes to these totals or
  their consumers. Never fix a failure by changing the frozen v1 expectations or
  refreshing them from live data. See `contracts/declared-totals-v1/README.md`.

## Frontend coding policy

- Frontend presentation is largely a prototype. UI edits must preserve the financial contract above.
- Every PR that changes the UI must include desktop screenshots in its
  description. Capture the affected views from the running application on
  the PR branch after completing the changes.
- Upload screenshots as GitHub attachments with `gh pr create --attach` or
  `gh pr edit --attach`. Use a GitHub CLI version that supports this flag. Never
  commit PR screenshots to the repository. Verify that the attachments render
  in the PR description before marking the PR ready for review.


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

- Keep schema changes in `db/migrations` as SQLx versioned `.up.sql` and
  `.down.sql` pairs. From the repository root, create a pair with
  `sqlx migrate add --source db/migrations --reversible <name>`.
- Never change a migration file that existed before the current PR. Treat those
  migrations as immutable, even when they have not been applied locally.
- When a PR's schema design changes, you must edit, consolidate, or remove the
  migrations introduced by that PR. Do not append a corrective migration for an
  obsolete step in the same PR. Before review, the PR must contain only the
  migrations needed for its final schema. Recreate isolated test databases that
  applied a discarded PR migration; do not preserve obsolete steps just to
  satisfy their checksums.
- Do not fold changes into the baseline or put manual schema scripts in
  `db/upgrades`.
- From the repository root, apply migrations with
  `sqlx migrate run --config sqlx.toml --source db/migrations`, inspect them with
  `sqlx migrate info --config sqlx.toml --source db/migrations`, and revert the
  latest version with
  `sqlx migrate revert --config sqlx.toml --source db/migrations`.
  Commands read the root `.env`; set `DATABASE_URL` to select another database.
- SQLx owns each migration transaction. Do not add `BEGIN` or `COMMIT` to the
  migration files. Keep each down migration limited to its up migration's changes.
- Test forward upgrades with existing records and the full down/up cycle in an
  isolated database. Preserve imported data when upgrading an existing database.
- The one-time transition from the consolidated baseline requires schema and
  data verification before aligning its checksum. Follow `db/README.md`;
  changing the checksum alone does not apply schema changes.
