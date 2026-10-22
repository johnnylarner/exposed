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

- The backend member summary owns mandatory declared GBP totals across all
  categories, including unknown category IDs. Frontends display those fields
  without recalculation or fallback sums.
- Current-term totals include records registered on or after
  `2024-07-09T00:00:00Z`. Historical totals include all records; undated records
  belong only to historical totals.
- Preserve exact decimal amounts and currencies. Do not convert currencies or
  pass amounts through JavaScript `Number`. Missing GBP amounts and unknown
  currencies make totals incomplete; known foreign currencies are excluded from
  GBP amounts without making totals incomplete.
- Preserve declaration counts and distinguish null amounts from zero. Label
  these as declared amounts, not earnings: declarations can include rates and
  benefits rather than money earned.
- Financial semantic changes require explicit user authorization and a description
  of their effects on included records/categories, currency, registration dates,
  missing values, precision, and labels. UI changes and refactoring must preserve
  these rules.

## Frontend coding policy

- Frontend presentation is largely a prototype. UI edits must preserve the financial contract above.

### Client and server

- Use Node.js 24+. From `frontend`, run `npm ci` and `npm run dev` for local
  development. Production uses `npm run build` followed by `node build`.
- Render initial page data through SvelteKit server loads. Keep Rust API access
  and response parsing in `frontend/src/lib/server` and `+page.server.ts`.
- Keep `EXPOSED_API_ORIGIN` on the server. Browser navigation and data requests
  go through the same-origin SvelteKit application, not directly to the Rust API.
- Pass typed, serializable page models to components. Keep browser APIs and SDK
  initialization in client hooks, event handlers, or `onMount` so SSR stays safe.
- Declare public configuration in `frontend/src/env.ts`. Use `dev` from
  `$app/env` for development-only behavior, including the design-system route.

### PostHog

- Initialize PostHog in `frontend/src/hooks.client.ts`, only outside development
  and when both public PostHog configuration values are present.
- Capture events in feature handlers, once per executed action. Preserve event
  names and property shapes for existing actions. Keep analytics out of shared UI
  components.
- Record metadata such as query length, entity type, and result position rather
  than raw search text or response payloads.
- Use `posthogLogger` for frontend diagnostic logs. Keep client error capture in
  the existing hook instead of adding duplicate exception reports.

### Design system

- Read the [component reference](frontend/src/lib/ui/README.md) and
  [composition recipes](frontend/src/lib/ui/COMPOSITION.md) before adding UI.
  The interactive reference is `/design-system` in development only.
- Import components and types from `frontend/src/lib/ui/index.ts` through the
  appropriate relative path. Load `frontend/src/lib/ui/styles.css` once in the
  root layout.
- Reuse components and typed variants. Use tokens for colors, spacing, and
  focus. Purple identifies MPs, teal identifies funders, and utility icons use
  the muted neutral color.
- Keep page arrangement, URLs, async work, and financial formatting in features.
  Shared UI accepts presentation models. Scope feature CSS to page arrangement
  and caller-authored content rather than component internals.
- Extend reusable patterns in `frontend/src/lib/ui`. Export public additions
  from the barrel, document their contracts, and add a gallery example. Cover
  changed behavior in `frontend/tests/design-system.spec.ts` or feature tests.

### UI verification

- Run `npm run check`, `npm run build`, and the affected Playwright tests from
  `frontend`. Gallery tests use a development server. Production tests use the
  built application.
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
