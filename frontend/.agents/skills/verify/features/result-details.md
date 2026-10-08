# Result details

Results show names in API order, distinguish MPs from funders, and expose funder categories through labels and tooltips. Names retain the spelling supplied by the API.

## Sub-features

- `results-names` preserves response order, spelling, and duplicate names.
- `results-highlights` marks literal query matches within names.
- `results-kind` distinguishes MPs, companies, individual funders, and unclassified funders.
- `results-tooltip` explains icons on focus or hover and closes with Escape.
- `results-layout` keeps names and category labels usable on a narrow screen.
- `entity-navigation` opens native member and funder detail links and preserves search parameters.
- `member-details` shows the latest 20 declarations with all funding occurrences.
- `funder-details` shows separate currency totals by latest stored party and up to ten recipients, including local missing-value counts.
- `results-limit` explains a per-kind result limit without claiming a total count.

## How to get to it (user POV)

- Type `John`, `McDonald's`, or `Aaron Banks` in search.
- Open a query link such as `/?q=John&strictness=1`.
- Hover or Tab to an entity icon to read its description.
- View the same results on a narrow browser window.

## Driving it with Playwright

Preconditions: Launch and Doctor passed. Use the repository fixtures for live MP, company, and individual-funder proof.

- Register `page.waitForResponse` for `/api/search` before filling the searchbox. Save `await response.json()` and require `await expect(page.locator(".entity-name")).toHaveText(body.entities.map(entity => entity.name));`. This proves display fidelity to that response, not the backend's ranking algorithm.
- Search `John`. Require two `getByRole("button", { name: "MP", exact: true })` controls, no `.entity-kind` elements, and `.entity-name mark` text of `John` for both results.
- Search `McDonald's`. Require the name unchanged and `.entity-kind-label` text `Company`. Search `Aaron Banks` and require `Individual`.
- With `Aaron Banks` visible, run `const icon = page.getByRole("button", { name: "Funder: Individual", exact: true }); await icon.focus();`. Require `.entity-tooltip` text `Funder: Individual`. Run `await icon.press("Escape");` and require zero tooltips. Blur, hover the icon, and then hover `.entity-tooltip`; the tooltip stays visible until the pointer leaves.
- Run `await page.setViewportSize({ width: 360, height: 780 });` and repeat the funder search. Require a visible category label and `await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)` to be true. Save the narrow screenshot with its response and ARIA snapshot.
- To cover duplicate names, markup-like text, unclassified types, or ten results of one type, use an appropriate isolated dataset or the existing controlled tests in `frontend/tests/search.spec.ts`. Record that the shipped fixtures do not cover these cases. At ten results of one kind, `.results-note` asks the user to narrow the search.

- Click the `John McDonnell` name link. Require a purple `MP` label, the `Recent declarations` heading, fixture category and funding values. Save the real `/api/members/{id}` response and compare declaration categories, dates, occurrence count and exact decimal strings with the UI. Unknown fields remain explicitly unavailable. Reload the path, then use browser Back and require the original query and slider value.
- Search `McDonald's`, click its name, and require a teal `Funder` label, `Declared support by party`, and `Top recipients`. Save the real `/api/funders/{uuid}` response and compare currency tables with it. Follow a recipient link to the member page and `Back to search` to restore the query and strictness. Middle-click a name link and require a new tab at its href.
- Repeat member and funder paths at 360px width and require no document overflow. Source names stay plain text. Capture screenshots and ARIA snapshots.
- Live fixtures have only complete GBP entries. SQLx detail tests and controlled browser tests cover duplicate precision, independent currencies, missing amounts/currencies, empty declarations, all-unknown totals, errors, not-found, timeout and cancellation. Label controlled proof separately.

## Gotchas

- Name links open `/members/{Parliament ID}` or `/funders/{UUID}`. Icon tooltip buttons are siblings of links. Direct paths require the frontend host’s SPA fallback.
- The API caps MPs and funders independently at 10. A combined response can contain 20 rows.
- Tied similarities do not provide a fixed name order. Compare with the actual response rather than inventing an alphabetical order.
- Test fixtures contain only two funder categories. They cannot prove every category or truncation behavior.
- The default API tests intercept requests. Their fabricated ordering is not evidence of live ranking.
