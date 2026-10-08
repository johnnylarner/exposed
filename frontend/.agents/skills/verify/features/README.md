# Exposed search verification map

Read this index before driving the app. These recipes describe the current search UI. Member and funder pages use native links. Pagination and feedback submission are not implemented.

## Baseline preconditions

- Run Launch and Doctor from [the verify skill](../SKILL.md).
- Use the owned frontend at `$VERIFY_BASE_URL` and the isolated fixture database.
- The fixtures contain MPs `John McDonnell` and `John Humphries`, company `McDonald's`, and individual funder `Aaron Banks`.
- Keep proof in a new subdirectory under `$VERIFY_EVIDENCE`. Keep evidence after teardown.
- Do not drive the main checkout's containers or share a browser session with another driver.

## Driving conventions

Use the Playwright `page` from the executable [search helper](../scripts/search.mjs). Recipes below contain actions to use inside a copy of that helper, with its browser setup, trace, and cleanup retained. Start each recipe at `/` unless it specifies a query URL.

Use roles and accessible names for controls. Result rows have no individual accessible names, so use the existing `.entity-row`, `.entity-name`, and `.entity-kind-label` selectors. Wait for the response and the resulting UI state instead of adding sleeps. Search starts after a 250 ms debounce and times out after 10 seconds.

## Proof and skip reporting

Record the feature ID, entry point, action, request, and result. Capture a screenshot and ARIA snapshot for each meaningful state. Keep the trace and real response body. Compare database snapshots for the absence of search writes.

Report only the entry points actually driven. The shipped helper proves part of `search-controls`; it does not claim the whole map. Existing tests that fulfill API requests prove frontend behavior with controlled responses. Label them separately from live integration proof.

## Features

- [Search controls](search-controls.md) covers typing, examples, Enter, keyboard access, and clearing.
- [Result details](result-details.md) covers names, order, highlighting, MP and funder context, tooltips, and narrow screens.
- [Threshold and shared URLs](threshold-and-urls.md) covers the slider, direct links, reloads, and URL restoration.
- [Empty, error, and pending searches](search-recovery.md) covers no matches, retry, cancellation, and unavailable service behavior.

Use `/maintain-verification-skill` to reconcile these files with app changes.
