## Worktree policy

- Before changing files for a new task, work in a dedicated Git worktree.
- If this chat already has a dedicated worktree, reuse it for follow-up work.
- Otherwise, create a sibling worktree with a unique branch and perform all
  edits, installs, and checks there.
- Never modify the primary checkout.
- Report the worktree path and branch when starting work.

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
