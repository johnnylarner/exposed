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
- When reporting completed and committed work, include a runnable `git diff`
  command covering the task's commits. Use `git -C` with the worktree path and
  explicit start/end commit references. Put the command in its own fenced `sh`
  code block with the copy-to-clipboard control so it is easy to copy and run.
- Keep Git history linear. Rebase feature branches onto the target branch and
  integrate changes with a fast-forward or squash; do not create merge commits.
