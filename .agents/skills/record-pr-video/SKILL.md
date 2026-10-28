---
name: record-pr-video
description: Record Exposed UI workflows for pull requests with a visible cursor, click highlights, and synchronized keyboard annotations. Use when preparing UI PR evidence or when asked for an annotated usage video.
---

# Record an annotated PR video

Follow the UI verification requirements in [AGENTS.md](../../../AGENTS.md).
Use the application built from the PR worktree. Choose a short workflow that
shows the changed interaction and its observable result. Use safe demonstration
data and identify fixture data in the PR description when applicable.

## Prepare the recording

Read `frontend/playwright.config.ts` and the affected browser tests for the
server setup and locators. Use the existing Playwright dependency in `frontend`.
Keep recording scripts and output in a temporary directory outside the repository.
Do not change application components to add recording annotations.

Create a dedicated browser context with video recording enabled before opening
the page. Set both the viewport and video size explicitly, for example to
1280 by 800, so the default video size does not shrink the annotations.
Use `recordVideo` for a standalone script or `video: 'on'` in a temporary
Playwright test configuration. Failure-only video settings discard successful demos.

## Annotate actions

Inject a recording-only overlay with `browserContext.addInitScript` before the
first navigation. Mount it when the document is ready and recreate it on full
navigation. Keep one overlay per document, outside the accessibility tree, with
`pointer-events: none`. Isolate its styling from the application, for example
with a shadow root. Keep it above page content without changing page layout.

- Draw a visible cursor at viewport coordinates from pointer movement. Do not
  assume that the browser's native pointer appears in the video.
- Move the pointer toward each target with intermediate steps before clicking.
  Use a real Playwright click and pulse a ring at the actual pointer-down
  coordinates for roughly 400 milliseconds. Read fresh target coordinates after
  scrolling or layout changes. Avoid DOM `click()` calls that bypass pointer events.
- Display keyboard actions in a high-contrast overlay away from the active
  control and result. Show shortcuts as one chord, such as `⌘K` or `Ctrl+K`,
  using the modifier actually sent. Label special keys such as `Enter`, `Tab`,
  `Escape`, and `Backspace`.
- Drive keyboard captions from the recording script's intended actions. Show
  each caption immediately before its matching Playwright operation and keep
  it visible long enough to read. A `fill()` operation does not emit a key event
  for each character. Caption it as text entry, or use sequential key presses
  when typing behavior is part of the demonstration.
- Show the entered text only for explicitly chosen, safe demo values. For
  sensitive input, use a generic caption such as `Enter password` and preserve
  the field's masking. Never collect arbitrary field values or clipboard contents
  into captions or recording logs. Omit flows whose page content exposes secrets.

Do not draw annotations for actions that were not executed. Stop and report a
failed action instead of editing the recording to imply success. For frames or
popups, verify that cursor coordinates and annotations appear in the recorded
page. Record a separate clip when one page's video cannot show the whole flow.

## Capture and inspect

Wait for visible application state with locators and assertions. Add short
presentation pauses after readiness checks, before actions, and on the final
result. Avoid long idle periods and abrupt cuts that hide the outcome.

Save the page's video handle before closing the browser context. Close the
context to finalize the video, then save the artifact. Playwright produces WebM.
If the upload path needs MP4, convert with an available video tool and preserve
the dimensions and readable text. Inspect the converted artifact too.

Play the complete video. Check that the initial state, cursor movement, click
highlights, text entry, shortcuts used by the flow, and final result are legible
and synchronized. Ensure overlays do not cover the demonstrated controls.
Check full navigations for missing overlays. Fix the recording and repeat when
an action or its result is unclear. Extracted frames can help inspect appearance
but do not establish timing or playback. Report any unverified playback.

## Attach to the PR

Check `gh pr create --help` or `gh pr edit --help` for `--attach` support.
Attach the video to the existing PR, or include it when creating the PR.
Use a body file for multiline descriptions and preserve unrelated PR content.
Open the PR in an available browser and play the attached video before reporting
that the attachment works. If upload or playback is unavailable, retain the
artifact, provide its path, and report the remaining step. Do not substitute a
local filesystem link for a GitHub attachment or commit the video to Git.
