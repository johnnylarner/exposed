# Empty, error, and pending searches

Search distinguishes no matches from an unavailable service. Users can edit an empty search, retry an error, and replace or clear a pending query.

## Sub-features

- `search-empty` explains that a completed query has no matches.
- `search-edit` focuses and selects the empty query for replacement.
- `search-error` exposes a failed request through an alert and retry button.
- `search-retry` resubmits the current query and recovers when the service returns.
- `search-pending` announces loading without showing old results as current.
- `search-cancel` prevents an earlier response from replacing a newer or cleared query.

## How to get to it (user POV)

- Search for `zzzzverificationnomatch` at threshold 1, then choose **Edit your search**.
- Search while the API is unavailable, then choose **Try again** after it returns.
- Type a new query or clear the input while a request is pending.

## Driving it with Playwright

Preconditions: Launch and Doctor passed. Only stop the API from this verification run. Keep the browser context open across the failure and recovery actions.

- Run `await page.goto("/?q=zzzzverificationnomatch&strictness=1");`. Require a real 200 response with `entities: []`, a heading starting with `No matches for`, and **Edit your search**. Run `await page.getByRole("button", { name: "Edit your search" }).click();`, then `await page.keyboard.type("John");`. Require input value `John` and both fixture MPs. This checks selection by replacing the old query through typing.
- In the worktree terminal, run `docker compose -f compose.yaml -f "$VERIFY_COMPOSE" -p "$VERIFY_PROJECT" stop exposed`. In the browser, run `await page.getByRole("searchbox").fill("John"); await page.getByRole("searchbox").press("Enter");`. Require `getByRole("alert")` to contain `Search couldn’t load` and capture the failed request plus the error screenshot. Enter also resubmits when `John` is already loaded.
- Start only that API with `docker compose -f compose.yaml -f "$VERIFY_COMPOSE" -p "$VERIFY_PROJECT" start exposed`. Wait for the real proxied search used in Doctor to succeed, then rerun Doctor. Click `getByRole("button", { name: "Try again" })` in the same browser page. Require both MPs from a new real response and no alert. Capture the before and after states in the same trace.
- For a pending request, observe `getByRole("region", { name: "Search results" })` with `aria-busy="true"` and the **Searching** heading. Replace `John` with `Aaron` before the first response completes, or click **Clear search**. Require that only the new result or the idle state remains after the old response finishes.
- The local API may be too fast to establish an overlapping-request race. Use the existing test named `a late response cannot replace a newer query or repopulate a cleared search` for deterministic frontend cancellation evidence. Label its intercepted responses. Do not claim a live race from two requests that finished sequentially.

## Gotchas

- The request timeout is 10 seconds. Allow at least 15 seconds for an error assertion.
- An empty response is HTTP 200. A connection error is not proof of the empty state.
- Retry bypasses the normal debounce and preserves the current query and threshold.
- A health check of the frontend page can pass while Rust is unavailable.
- Always run the skill's Cleanup after a failed recovery attempt. Do not leave a stopped or restarted shared API behind.
