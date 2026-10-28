# Project summary and structure

The goal of this project is to make declaration data easy to search, browse and interpret.

- `exposed/` contains the rust application and CLI code that powers the project.
- `frontend/` contains a simple Svelte application

## Domain terminology

Use these meanings consistently in code, documentation, and UI copy.

- **Declarations** are records of financial interests registered by a member.
  A declaration belongs to a category and can contain zero, one, or multiple
  funding entries. A declaration count is not a payment or funding-entry count.
  Registration dates do not necessarily describe when money changed hands.
- **Members** are people identified by their Parliament member ID. Public MP
  pages use that identity. Stored records can include former members, and the
  latest stored party or constituency does not establish affiliation at the
  time of a declaration.
- **Funding** refers to the payments, support, rates, loans, gifts, or other
  benefits recorded within declarations. A funding entry is one source
  occurrence, even when another entry has identical values. Funding can have
  no known amount or resolved funder. Support linked to a member can go to a
  party organisation rather than to the member personally.
- **Rankings** are ordered comparisons of declared GBP amounts in the imported
  data, not measures of income or wealth.

**Note**: users may use the word _earnings_ when prompting. You should understand this to mean all funding sources.

## Worktree policy

- For read-only tasks (investigating, reading, summarizing, or describing code),
  work on the current branch without creating a worktree.
- For tasks that require changing code or other files, use [`worktrunk`](https://worktrunk.dev):
  - `wt switch --create <branch-name> --base main`
- Never modify the primary checkout without explicit permission.

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

All frontend changes must follow this policy, including prototype UI. UI edits
must preserve the financial totals contract above. The design-system rules are
required acceptance criteria for new UI and changes to existing UI.

### UI copy scope

- Add only the visible text needed for the requested interaction or data.
  Do not add unrequested subtitles, helper paragraphs, explanatory labels,
  badges, or repeated context to make a feature feel complete.
- Reuse existing wording. Keep required control labels, accessible names,
  error messages, and the financial wording required above.
- Before committing UI changes, review each added visible string against the
  request. Remove optional copy that the user did not request. A component
  example is not permission to copy its explanatory text into a feature.

| Rule | Enforcement |
| --- | --- |
| Omit unrequested visible UI copy. | Review added strings in the diff against the user's request. This is a judgment check; types and automated tests cannot determine whether copy was requested. |

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

- You must read the [component reference](frontend/src/lib/ui/README.md) and
  [composition recipes](frontend/src/lib/ui/COMPOSITION.md) before adding or
  changing UI. Follow their component contracts and composition patterns.
  The interactive reference is `/design-system` in development only.
- You must import shared UI components and types from
  `frontend/src/lib/ui/index.ts` through the appropriate relative path. Load
  `frontend/src/lib/ui/styles.css` once in the root layout. Do not import
  component internals or internal stylesheets directly.
- You must reuse existing components, composition patterns, and typed variants.
  Do not recreate shared controls or patterns in feature components. When a
  reusable capability is missing, extend the design system before using it
  in a feature.
- You must use design-system tokens for colors, spacing, typography, and focus.
  Do not substitute hard-coded values for those tokens. Purple identifies MPs,
  teal identifies funders, and utility icons use the muted neutral color.
  Actions and focus indicators must stay neutral in every entity context.
- Keep page arrangement, URLs, async work, and financial formatting in features.
  Shared UI must accept presentation models without interpreting money or
  fetching data. Feature CSS must be limited to page arrangement and
  caller-authored content. Do not target shared component internals or override
  their appearance through global selectors or replacement classes. Use the
  documented typed variants and additive root hooks.
- You must implement reusable additions in `frontend/src/lib/ui`, export them
  from the barrel, document their contracts, and add a gallery example. Cover
  changed behavior in `frontend/tests/design-system.spec.ts` or feature tests.

| Rule | Enforcement |
| --- | --- |
| Follow the design-system contracts for every UI change. | Before committing, review the diff against the component reference and composition recipes. Remove duplicate UI, direct internal imports, hard-coded token values, and overrides of component internals. |
| Complete reusable additions in the design system. | Verify the barrel export, documented contract, gallery example, and affected tests before marking the PR ready for review. |

### UI verification

- Run `npm run check`, `npm run build`, and the affected Playwright tests from
  `frontend`. Gallery tests use a development server. Production tests use the
  built application.
- Match the evidence in the PR description to the type of UI change:
  - Aesthetic-only changes, such as sizes, colours, and positions, require
    desktop screenshots of the affected views. No video is required.
  - Functional changes and new features, including anything that requires
    steps to test, require a short desktop usage video of the affected workflow.
    Show the starting state, relevant actions, and resulting state, with
    deliberate pauses so reviewers can follow. Screenshots are optional
    supplements. PRs with both aesthetic and functional changes require video.
  Capture evidence from the running application on the PR branch after
  completing the changes.
- For videos, use the [PR video recording skill](.agents/skills/record-pr-video/SKILL.md).
  Show a visible cursor, a brief highlight at each click location, and readable
  overlays for simulated keyboard inputs, including shortcuts and special keys.
  Show entered text only when it is safe to disclose. Keep annotations synchronized
  with actions and clear of the controls being demonstrated.
- Upload screenshots and videos as GitHub attachments with `gh pr create --attach` or
  `gh pr edit --attach`. Use a GitHub CLI version that supports this flag. Never
  commit PR screenshots or recordings to the repository. Verify that screenshots
  render and videos play in the PR description before marking the PR ready for
  review. If uploading or verification is unavailable, report the blocker and
  provide the local evidence artifact.


## Git history

- After completing and validating a task, commit its changes on the task's work
  branch, push the branch, and open a pull request against the target branch
  without waiting for a separate request. If the branch already has an open PR,
  update it instead of creating a duplicate.
- Before reporting completion, verify that the remote branch matches local
  `HEAD` and that the open PR targets the intended base and includes that commit.
  Include the PR URL and validation results in the final response.
- If pushing or opening the PR fails, report the blocker and the remaining step.
  Do not describe a local commit alone as completed delivery. Follow explicit
  user instructions to stop before pushing or opening a PR.
- Never commit directly on `main`.
- Keep Git history linear. Rebase feature branches onto the target branch and
  integrate changes with a fast-forward or squash; do not create merge commits.

| Rule | Enforcement |
| --- | --- |
| Deliver validated changes as a pushed branch with an open PR. | Agent completion check against the remote branch and `gh pr view`; include the verified PR URL in the final response. This is a workflow rule, not an automated gate. CI cannot observe work that was never pushed. |

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
