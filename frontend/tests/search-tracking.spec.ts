import type { Page } from "@playwright/test";
import { lordsFixture } from "./lords-fixtures";
import { gunzipSync } from "node:zlib";
import { expect, test } from "./backend";

test.use({
  analytics: true,
  userAgent:
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36",
});

async function captureEvents(page: Page) {
  const events: { event: string; properties: Record<string, unknown> }[] = [];
  await page.route("https://posthog.test/**", async (route) => {
    const request = route.request();
    const body = request.postDataBuffer();
    if (body && new URL(request.url()).pathname === "/e/") {
      const url = new URL(request.url());
      const decoded =
        body[0] === 0x1f && body[1] === 0x8b
          ? gunzipSync(body).toString()
          : url.searchParams.get("compression") === "base64"
            ? Buffer.from(
                new URLSearchParams(body.toString()).get("data")!,
                "base64",
              ).toString()
            : body.toString();
      const payload = JSON.parse(decoded);
      events.push(
        ...(payload.batch ?? (Array.isArray(payload) ? payload : [payload])),
      );
    }
    await route.fulfill({ json: {} });
  });
  await page.addInitScript(() => {
    Object.defineProperty(navigator, "webdriver", { get: () => false });
    Object.defineProperty(navigator, "userAgentData", { get: () => undefined });
  });
  return events;
}

test("each executed search emits one event, including typeahead, examples, Enter and retry", async ({
  backend,
  page,
}) => {
  const events = await captureEvents(page);
  let fail = false;
  const searches: string[] = [];
  await backend.route("**/api/search?*", (route) => {
    searches.push(new URL(route.request().url()).searchParams.get("term")!);
    return fail
      ? route.fulfill({ status: 503, body: "service unavailable" })
      : route.fulfill({ json: { entities: [] } });
  });
  await page.goto("/search?q=First");
  const input = page.getByRole("searchbox");
  await input.press("Enter");
  await expect
    .poll(
      () => events.filter(({ event }) => event === "search_submitted").length,
      { timeout: 10000 },
    )
    .toBe(1);
  await page.getByRole("button", { name: "Clear search" }).click();
  await input.fill("jo");
  await input.press("Enter");
  await input.fill("Canceled");
  await input.fill("");
  await input.fill("  Jo😀  ");
  await expect(
    page.getByRole("heading", { name: "No matches for “Jo😀”" }),
  ).toBeVisible();
  await expect
    .poll(
      () => events.filter(({ event }) => event === "search_submitted").length,
      { timeout: 10000 },
    )
    .toBe(2);
  await page.getByRole("button", { name: "Clear search" }).click();
  await page.getByRole("button", { name: "John", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "No matches for “John”" }),
  ).toBeVisible();
  await input.fill("Enter");
  await input.press("Enter");
  await expect(
    page.getByRole("heading", { name: "No matches for “Enter”" }),
  ).toBeVisible();
  fail = true;
  await input.fill("Retry");
  await expect(page.getByRole("alert")).toBeVisible();
  fail = false;
  await page.getByRole("button", { name: "Try again" }).click();
  await expect(
    page.getByRole("heading", { name: "No matches for “Retry”" }),
  ).toBeVisible();
  await expect
    .poll(
      () =>
        events
          .filter(({ event }) => event === "search_submitted")
          .map(({ properties }) => properties.query_length),
      { timeout: 10000 },
    )
    .toEqual([5, 3, 4, 5, 5, 5]);
  expect(searches).toEqual([
    "First",
    "First",
    "Jo😀",
    "John",
    "Enter",
    "Retry",
    "Retry",
  ]);
});

test("Lords identity captures one view event per executed action in the shared detail shell", async ({
  backend,
  page,
}) => {
  const events = await captureEvents(page);
  await backend.route("**/api/lords/2175", (route) =>
    route.fulfill({ json: lordsFixture }),
  );
  await page.goto("/lords/2175?q=Example");
  await page
    .getByRole("link", { name: "Interest history", exact: true })
    .click();
  await expect(
    page.getByRole("link", { name: "Interest history", exact: true }),
  ).toHaveAttribute("aria-current", "page");
  await page.getByRole("link", { name: "Summary", exact: true }).click();
  await expect(
    page.getByRole("link", { name: "Summary", exact: true }),
  ).toHaveAttribute("aria-current", "page");
  await expect
    .poll(
      () =>
        events
          .filter(({ event }) => event === "entity_view_selected")
          .map(({ properties }) => ({
            entity_type: properties.entity_type,
            selected_view: properties.selected_view,
          })),
      { timeout: 10000 },
    )
    .toEqual([
      { entity_type: "lord", selected_view: "history" },
      { entity_type: "lord", selected_view: "summary" },
    ]);
});
