import { expect, test } from "./backend";

for (const colorScheme of ["light", "dark"] as const) {
  test(`${colorScheme} initial canvas and metadata work without JavaScript`, async ({
    browser,
    baseURL,
  }) => {
    const context = await browser.newContext({
      javaScriptEnabled: false,
      colorScheme,
    });
    const page = await context.newPage();
    await page.goto(`${baseURL}/`);
    const expected =
      colorScheme === "light" ? "rgb(245, 245, 244)" : "rgb(0, 0, 0)";
    await expect(page.locator("html")).toHaveCSS("background-color", expected);
    await expect(page.locator("html")).toHaveCSS("color-scheme", colorScheme);
    await expect(
      page.locator(
        `meta[name="theme-color"][media="(prefers-color-scheme: ${colorScheme})"]`,
      ),
    ).toHaveAttribute(
      "content",
      colorScheme === "light" ? "#f5f5f4" : "#000000",
    );
    let blockedStylesheets = 0;
    const initial = await context.newPage();
    await initial.route("**/*.css", (route) => {
      blockedStylesheets += 1;
      return route.abort();
    });
    await initial.goto(`${baseURL}/`);
    expect(blockedStylesheets).toBeGreaterThan(0);
    await expect(initial.locator("html")).toHaveCSS(
      "background-color",
      expected,
    );
    await expect(initial.locator("html")).toHaveCSS(
      "color-scheme",
      colorScheme,
    );
    await context.close();
  });
}
