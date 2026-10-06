# Threshold and shared URLs

Users can lower the similarity threshold to broaden a search and share the query and threshold through the page URL. Opening the link restores both controls.

## Sub-features

- `threshold-slider` refreshes a search when the user moves the slider.
- `threshold-range` supports values from 0 to 1 in steps of 0.01.
- `url-query` restores a query from `q`.
- `url-threshold` restores, rounds, or defaults `strictness`.
- `url-reload` preserves the visible query and threshold after reload.
- `url-history` restores values when the browser returns to a previous page URL.

## How to get to it (user POV)

- Search for a name and adjust **Similarity threshold** with the mouse or arrow keys.
- Open a shared `/?q=John&strictness=0.65` link.
- Reload the page, or navigate away and use the browser's Back button.

## Driving it with Playwright

Preconditions: Launch and Doctor passed. Retain the trace and response bodies for each threshold.

- Run `await page.goto("/?q=John&strictness=0.65");`. Require searchbox value `John`, slider value `0.65`, `.threshold-label output` text `0.65`, and a real request with `strictness=0.65`. Compare rendered names with that response.
- Run `const slider = page.getByRole("slider", { name: "Similarity threshold" }); await slider.focus(); await slider.press("ArrowLeft");`. Require slider value `0.64`, URL `strictness=0.64`, and a refreshed API response. Use the native key action, not a JavaScript assignment to the slider's value.
- Press Home on the focused slider. Require `0` and all four fixture entities. Press End and require `1`; for `John`, only the two MPs remain. Capture both responses and screenshots.
- Run `await page.reload();` and require restored query, slider, and results. Run `await page.goto("/?q=John&strictness=invalid");` and require a slider and API value of `1`. For `strictness=0.654`, require `0.65`.
- Start on `/?q=John&strictness=0.65`, navigate with `await page.goto("/?q=Aaron&strictness=1");`, then run `await page.goBack();`. Require the earlier query, threshold, and resulting names. This exercises browser navigation without changing internal app state.
- Save the final URL with each proof. Query text alone does not establish that the threshold reached the API.

## Gotchas

- Typing and slider changes use `replaceState`. They do not create a new browser-history entry per edit.
- A direct URL can retain an invalid raw parameter even though the control and request use the default. Check the controls and request, not only the URL text.
- Lower thresholds can still produce capped results. Compare actual responses rather than assuming strict set inclusion in a larger dataset.
- Clear search retains the selected threshold.
