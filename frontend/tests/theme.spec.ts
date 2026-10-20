import { expect, test } from "./backend";
import type { Locator } from "@playwright/test";

async function contrast(
  locator: Locator,
  {
    foreground = "color",
    background = "self",
  }: {
    foreground?: "color" | "borderTopColor";
    background?: "self" | "page";
  } = {},
) {
  const colors = await locator.evaluate(
    (element, { foreground, background }) => ({
      foreground: getComputedStyle(element)[foreground],
      background: getComputedStyle(
        background === "page" ? document.documentElement : element,
      ).backgroundColor,
    }),
    { foreground, background },
  );
  function luminance(color: string) {
    const channels = color
      .replace(/[^\d.,]/gu, "")
      .split(",")
      .map(Number);
    return channels.slice(0, 3).reduce((total, channel, index) => {
      const value = channel / 255;
      const linear =
        value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
      return total + linear * [0.2126, 0.7152, 0.0722][index];
    }, 0);
  }
  const values = [luminance(colors.foreground), luminance(colors.background)];
  return (Math.max(...values) + 0.05) / (Math.min(...values) + 0.05);
}

for (const colorScheme of ["light", "dark"] as const) {
  test(`${colorScheme} follows the system across pages and interactive search states`, async ({
    page,
    backend,
  }) => {
    await page.emulateMedia({ colorScheme });
    await backend.route("**/api/search?*", (route) =>
      route.fulfill({
        json: {
          entities: [
            {
              id: "1",
              name: "John Cooper",
              kind: "MP",
              funder_kind: null,
              match_source: { kind: "name" },
            },
          ],
        },
      }),
    );
    await backend.route("**/api/members/1", (route) =>
      route.fulfill({
        json: {
          member: {
            id: "1",
            name: "John Cooper",
            party_id: 1,
            party_name: "Labour",
            membership_from: "Example constituency",
            is_current_commons: true,
          },
          declarations: [],
          summary: {
            currencies: [],
            entry_count: 0,
            unknown_currency_count: 0,
            first_registered_at: null,
            last_registered_at: null,
            undated_declaration_count: 0,
          },
        },
      }),
    );
    await page.goto("/members/1/history");
    const date = page.getByLabel("Registered from");
    expect(
      await contrast(date, { foreground: "borderTopColor" }),
    ).toBeGreaterThanOrEqual(3);
    expect(await contrast(date)).toBeGreaterThanOrEqual(4.5);
    await expect(date).toHaveCSS("color-scheme", colorScheme);
    await date.focus();
    await expect(date).toHaveCSS("outline-style", "solid");
    for (const path of ["/", "/methodology", "/search"]) {
      await page.goto(path);
      await expect(page.locator("html")).toHaveCSS("color-scheme", colorScheme);
      expect(await contrast(page.locator("html"))).toBeGreaterThanOrEqual(4.5);
    }
    const search = page.getByRole("searchbox");
    await search.fill("John");
    await expect(page.locator("mark")).toHaveText("John");
    expect(
      await contrast(page.locator("mark"), { background: "page" }),
    ).toBeGreaterThanOrEqual(4.5);
    expect(
      await contrast(page.locator(".search-field"), {
        foreground: "borderTopColor",
      }),
    ).toBeGreaterThanOrEqual(3);
    let finishLoading = () => {};
    const loading = new Promise<void>((resolve) => {
      finishLoading = resolve;
    });
    await backend.route("**/api/search?*", async (route) => {
      await loading;
      await route.fulfill({ json: { entities: [] } });
    });
    await search.fill("Loading");
    await expect(page.locator(".loading-rows")).toBeVisible();
    finishLoading();
    await expect(page.locator(".message-state")).toBeVisible();
    await backend.route("**/api/search?*", (route) =>
      route.fulfill({ json: { entities: [] } }),
    );
    await search.fill("Absent");
    await expect(page.locator(".message-state")).toBeVisible();
    await backend.route("**/api/search?*", (route) =>
      route.fulfill({ status: 500 }),
    );
    await search.fill("Error");
    const retry = page.getByRole("button", { name: "Try again" });
    await expect(retry).toBeVisible();
    await retry.focus();
    await expect(retry).toHaveCSS("outline-style", "solid");
    await retry.hover();
    expect(await contrast(retry)).toBeGreaterThanOrEqual(4.5);
  });
}
