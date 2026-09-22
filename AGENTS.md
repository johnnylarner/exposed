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
- When reporting completed and committed work, include a runnable command comparing
  `main` with the latest commit on the work branch:
  `git -C <worktree-path> diff main <latest-commit-sha>`.
  Resolve the commit SHA after committing the work. Put the command in its own
  fenced `sh` code block with the copy-to-clipboard control.
- Keep Git history linear. Rebase feature branches onto the target branch and
  integrate changes with a fast-forward or squash; do not create merge commits.

## Development database migrations

- While the project is in development, keep a single baseline migration at
  `db/migrations/20260915000000_initial.sql`. Fold schema changes into that file
  without adding migration files or incrementing the version.
- For an existing database, inspect its applied schema and apply the necessary
  changes in place to preserve imported data. SQLx rejects an edited baseline's
  checksum; verify the live schema before updating its recorded checksum using
  the procedure in `db/README.md` or reporting a migration complete.
