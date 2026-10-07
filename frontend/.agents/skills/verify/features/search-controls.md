# Search controls

Users find MPs and funders by entering a name or choosing an example. Keyboard controls provide access to search and clear it without leaving the page.

## Sub-features

- `search-input` starts search after three trimmed characters and preserves the query in the URL.
- `search-submit` submits a valid query with Enter.
- `search-examples` starts searches from both idle example buttons.
- `search-keyboard` reaches search through `/` and the skip link.
- `search-clear` clears through the button or Escape and restores the idle state.
- `search-home` returns to the initial page through the Exposed wordmark.

## How to get to it (user POV)

- Open `/` and click the input labeled **Search MPs and funders**.
- Press `/` outside a text control, or Tab to **Skip to search** and press Enter.
- Choose **John** or **HSBC** below **Start with a name**.
- Press Enter with a valid query in the input.
- Choose **Clear search**, press Escape in the input, or choose the **Exposed home** link.

## Driving it with Playwright

Preconditions: Launch and Doctor passed with the fixture database. Use a fresh browser context and the default threshold of 1.

- Run `PLAYWRIGHT_BROWSERS_PATH=0 frontend/.agents/skills/verify/scripts/search.mjs "$VERIFY_BASE_URL" "$VERIFY_EVIDENCE/search"` from the repository root. It proves the slash shortcut, short-query Enter, typed `John`, and the clear button against the live backend. Its `proof.json` lists those entry points.
- To check valid Enter, run `await page.getByRole("searchbox").fill("John"); await page.getByRole("searchbox").press("Enter");`. Require **2 results shown** and a real `/api/search` response with `term=John`, `max_entries=10`, and `strictness=1`.
- From `/`, run `await page.getByRole("button", { name: "John", exact: true }).click();`. Require both fixture MPs and a focused searchbox. Repeat from `/` with the **HSBC** button and require the empty state for `HSBC`.
- After typing `John`, run `await page.getByRole("searchbox").press("Escape");`. Require an empty, focused input, no `.entity-row`, no `q` URL parameter, and **Start with a name**.
- From a fresh `/` load, run `await page.keyboard.press("Tab"); await expect(page.getByRole("link", { name: "Skip to search" })).toBeFocused(); await page.keyboard.press("Enter");`. Require the `#search` fragment and the main search section in view.
- From populated results, run `await page.getByRole("link", { name: "Exposed home" }).click();`. Require `/`, an empty input, and **Start with a name**.
- Capture screenshots and ARIA snapshots after the action, alongside the trace. Compare the database snapshots from the skill's Drive section.

## Gotchas

- `/` inside the input types a slash. It focuses search only when focus is outside an editable control.
- Escape clears only when the search input has focus. On an entity icon it dismisses a tooltip.
- Spaces do not count toward the minimum length. The API receives the trimmed query.
- Clearing removes `q` but preserves a previously selected `strictness`.
- **HSBC** is intentionally absent from the fixtures. Its example button still needs to start a real search.
