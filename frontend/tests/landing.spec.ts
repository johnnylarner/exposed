import { expect, test } from "./backend";

test("footer displays the build version on public pages", async ({ page }) => {
  const version = process.env.PUBLIC_APP_VERSION || "development";
  for (const path of ["/", "/methodology", "/search"]) {
    await page.goto(path);
    await expect(
      page
        .getByRole("contentinfo")
        .getByText(`Version ${version}`, { exact: true }),
    ).toBeVisible();
  }
});

test("landing and methodology navigate without backend requests", async ({
  page,
  backend,
}) => {
  const requests: string[] = [];
  await backend.route("**/api/*", async (route) => {
    requests.push(route.request().url());
    await route.fulfill({ status: 500 });
  });
  await page.goto("/");
  await expect(page).toHaveTitle("Exposed");
  await expect(page.getByRole("heading", { level: 1 })).toHaveText(
    "Explore MPs' declared financial interests and the people and organisations behind them.",
  );
  await expect(page.getByRole("link", { name: "Exposed home" })).toHaveCSS(
    "color",
    "rgb(182, 123, 229)",
  );
  await expect(
    page.locator("#landing h1").getByText("MPs", { exact: true }),
  ).toHaveCSS("color", "rgb(182, 123, 229)");
  await expect(
    page
      .locator("#landing h1")
      .getByText("financial interests", { exact: true }),
  ).toHaveCSS("color", "rgb(100, 207, 181)");
  await page.keyboard.press("Tab");
  await expect(
    page.getByRole("link", { name: "Skip to content" }),
  ).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.locator("#landing")).toBeFocused();
  await page
    .locator("#landing")
    .getByRole("link", { name: "Methodology", exact: true })
    .click();
  await expect(page).toHaveURL(/\/methodology$/u);
  await expect(page).toHaveTitle("Methodology · Exposed");
  await expect(
    page.getByRole("link", {
      name: "Members API",
    }),
  ).toHaveAttribute("href", "https://members-api.parliament.uk/index.html");
  await page.goto("/methodology");
  await page.keyboard.press("Tab");
  await page.keyboard.press("Enter");
  await expect(page.locator("#methodology")).toBeFocused();
  await page.getByRole("link", { name: "Exposed home" }).click();
  await page.getByRole("link", { name: "Search MPs and funders" }).click();
  await expect(page).toHaveURL(/\/search$/u);
  await expect(page.getByRole("searchbox")).toHaveValue("");
  expect(requests).toEqual([]);
});

for (const viewport of [
  { width: 1440, height: 900 },
  { width: 1280, height: 720 },
  { width: 390, height: 844 },
  { width: 320, height: 568 },
]) {
  test(`landing fits ${viewport.width}px viewport`, async ({ page }) => {
    await page.setViewportSize(viewport);
    await page.goto("/");
    await page.evaluate(() => document.fonts.ready);
    expect(
      await page.evaluate(() => ({
        width: document.documentElement.scrollWidth,
        height: document.documentElement.scrollHeight,
      })),
    ).toEqual(viewport);
    for (const link of await page.locator("#landing a").all()) {
      const bounds = await link.boundingBox();
      expect(bounds?.height).toBeGreaterThanOrEqual(44);
    }
    await page.locator("#landing h1").evaluate((heading) => {
      heading.style.fontSize = "64px";
    });
    expect(
      await page.evaluate(() => document.documentElement.scrollWidth),
    ).toBe(viewport.width);
  });
}

for (const { query, canonical, status } of [
  { query: "?q=", canonical: "/search?q=", status: 307 },
  { query: "?strictness=0.42", canonical: "/search", status: 308 },
  {
    query: "?q=Example&strictness=0.42&extra=&q=second",
    canonical: "/search?q=Example&extra=&q=second",
    status: 308,
  },
  {
    query: "?q=&strictness=&extra=%20",
    canonical: "/search?q=&extra=+",
    status: 308,
  },
]) {
  test(`legacy root preserves ${query} without JavaScript`, async ({
    browser,
    baseURL,
    backend,
    request,
  }) => {
    await backend.route("**/api/search?*", (route) =>
      route.fulfill({ json: { entities: [] } }),
    );
    const response = await request.get(`/${query}`, { maxRedirects: 0 });
    expect(response.status()).toBe(status);
    expect(response.headers().location).toBe(canonical);
    const context = await browser.newContext({
      javaScriptEnabled: false,
      baseURL,
    });
    try {
      const page = await context.newPage();
      await page.goto(`/${query}`);
      expect(new URL(page.url()).pathname + new URL(page.url()).search).toBe(
        canonical,
      );
      await expect(page.getByRole("searchbox")).toHaveValue(
        new URLSearchParams(query).get("q") ?? "",
      );
    } finally {
      await context.close();
    }
  });
}

test("unrelated root parameters retain landing and unknown routes remain not found", async ({
  page,
}) => {
  await page.goto("/?utm_source=test");
  await expect(page.locator("#landing")).toBeVisible();
  await page.goto("/missing");
  await expect(
    page.getByRole("heading", { name: "Entity not found" }),
  ).toBeVisible();
});

test("production hides the design system route and footer link", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("contentinfo").getByRole("link", { name: "Design system" }),
  ).toHaveCount(0);
  await page.evaluate(() => {
    const link = document.createElement("a");
    link.href = "/design-system";
    link.textContent = "Open development reference";
    document.body.append(link);
  });
  await page.getByRole("link", { name: "Open development reference" }).click();
  await expect(page).toHaveURL(/\/design-system$/u);
  await expect(
    page.getByRole("heading", { name: "Design system", exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("heading", { name: "404", exact: true }),
  ).toBeVisible();
  const response = await page.goto("/design-system");
  expect(response?.status()).toBe(404);
  await expect(page.locator("#design-system")).toHaveCount(0);
});
