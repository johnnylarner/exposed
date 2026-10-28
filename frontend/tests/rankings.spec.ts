import { expect, test } from "./backend";
import { completeRankingFixture, rankingFixture } from "./rankings-fixtures";

test("rankings SSR includes exact top ten and category switching is URL backed", async ({
  page,
  backend,
  request,
  baseURL,
}) => {
  let reads = 0;
  await backend.route("**/api/rankings?*", async (route) => {
    reads += 1;
    await route.fulfill({ json: rankingFixture });
  });
  const ssr = await request.get(`${baseURL}/rankings`);
  const html = await ssr.text();
  expect(html).toContain("£9,007,199,254,740,993.123456789");
  expect(html).toContain("Member 10");
  await page.goto("/");
  await page.getByRole("link", { name: "Rankings", exact: true }).click();
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("Rankings");
  const rows = page.getByRole("list", { name: "MPs rankings" });
  await expect(rows.getByRole("listitem")).toHaveCount(10);
  await expect(
    rows.getByRole("link", { name: "Member 1", exact: true }),
  ).toHaveAttribute("href", "/members/1?period=current-term");
  await expect(rows).toContainText("£9,007,199,254,740,993.123456789");
  await expect(rows).toContainText("Incomplete");
  await expect(rows).toContainText("Example Party · Example constituency");
  const before = reads;
  const individuals = page.getByRole("link", {
    name: "Individuals",
    exact: true,
  });
  await individuals.focus();
  await page.keyboard.press("Enter");

  await expect(individuals).toHaveAttribute("aria-current", "page");
  await expect(
    page.getByRole("list", { name: "Individuals rankings" }),
  ).toContainText("£0.00");
  await expect(
    page.getByRole("combobox", { name: "Reporting period" }),
  ).toHaveValue("current-term");
  await page.getByRole("link", { name: "Companies", exact: true }).click();
  await expect(
    page.getByRole("link", { name: "Example company" }),
  ).toHaveAttribute(
    "href",
    "/funders/10000000-0000-0000-0000-000000000002?period=current-term",
  );
  await expect(
    page.getByRole("list", { name: "Companies rankings" }),
  ).toContainText("−£12.3456789");
  await page.getByRole("link", { name: "Parties", exact: true }).click();
  const parties = page.getByRole("list", { name: "Parties rankings" });
  await expect(parties).toContainText("Amount unavailable");
  await expect(parties.getByRole("link")).toHaveCount(0);
  await expect(
    page.getByText("Support declared by MPs, grouped by their latest party."),
  ).toBeVisible();
  expect(reads).toBeGreaterThan(before);
  await expect(page).toHaveURL(/category=parties/u);
  await expect(
    page.getByRole("button", { name: /All records|Show more|Replay/u }),
  ).toHaveCount(0);
});

test("rankings fit narrow viewports and honor reduced motion", async ({
  page,
  backend,
}) => {
  await backend.route("**/api/rankings?*", async (route) =>
    route.fulfill({ json: completeRankingFixture() }),
  );
  await page.setViewportSize({ width: 320, height: 900 });
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/rankings");
  await expect(
    page.getByRole("list", { name: "MPs rankings" }).getByRole("listitem"),
  ).toHaveCount(10);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(
    320,
  );
  const row = page.getByRole("listitem").first();
  await expect(row).toHaveCSS("animation-name", "none");
  await page.getByRole("button", { name: "Show more" }).click();
  await expect(page.getByRole("listitem")).toHaveCount(20);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(
    320,
  );
  await page.getByRole("link", { name: "Companies", exact: true }).click();
  await expect(page.getByRole("listitem").first()).toHaveCSS(
    "animation-name",
    "none",
  );
});

test("rankings report unavailable and empty snapshots", async ({
  page,
  backend,
}) => {
  await backend.route("**/api/rankings?*", async (route) =>
    route.fulfill({ status: 503 }),
  );
  await page.goto("/rankings");
  await expect(page.getByRole("alert")).toContainText(
    "Rankings couldn’t load. Try again.",
  );
  await backend.route("**/api/rankings?*", async (route) =>
    route.fulfill({ json: { ...rankingFixture, members: [] } }),
  );
  await page.reload();
  await expect(page.getByText("No declarations found.")).toBeVisible();
});

test("rankings keep committed scope during rapid toolbar changes", async ({
  page,
  backend,
}) => {
  const delayed = Promise.withResolvers<void>();
  await backend.route("**/api/rankings?*", async (route) => {
    const period = new URL(route.request().url()).searchParams.get("period");
    if (period === "current-year") await delayed.promise;
    await route.fulfill({ json: { ...rankingFixture, period } });
  });
  await page.setViewportSize({ width: 1280, height: 400 });
  await page.goto("/rankings?q=Example&from=2020-01-01&page=2");
  const select = page.getByRole("combobox", { name: "Reporting period" });
  await page.evaluate(() => scrollTo(0, 150));
  await select.focus();
  await select.selectOption("current-year");
  await expect(page.locator("#rankings")).toHaveAttribute("aria-busy", "true");
  await expect(select).toHaveValue("current-term");
  try {
    await page.getByRole("link", { name: "Companies", exact: true }).click();
    await expect(page).toHaveURL(/category=companies&period=current-term/);
    await expect(select).toHaveValue("current-term");
    expect(await page.evaluate(() => scrollY)).toBe(
      await page.evaluate(() =>
        Math.min(150, document.documentElement.scrollHeight - innerHeight),
      ),
    );
  } finally {
    delayed.resolve();
  }
  await expect(
    page.getByRole("list", { name: "Companies rankings" }),
  ).toBeVisible();
  await select.selectOption("historical");
  await expect(select).toHaveValue("historical");
  await expect(page).toHaveURL(/q=Example.*period=historical/);
  const url = new URL(page.url());
  expect(url.searchParams.has("from")).toBe(false);
  expect(url.searchParams.has("page")).toBe(false);
});

for (const [category, label, name] of [
  ["members", "MPs", "Member"],
  ["individuals", "Individuals", "Individual"],
  ["companies", "Companies", "Company"],
  ["parties", "Parties", "Party"],
]) {
  test(`${label} reveal the complete ordered snapshot with stable rows and final keyboard focus`, async ({
    page,
    backend,
  }) => {
    let reads = 0;
    await backend.route("**/api/rankings?*", async (route) => {
      reads += 1;
      await route.fulfill({ json: completeRankingFixture() });
    });
    const requests: string[] = [];
    page.on("request", (request) => requests.push(request.url()));
    await page.goto(`/rankings?category=${category}`);
    const list = page.getByRole("list", { name: `${label} rankings` });
    const items = list.getByRole("listitem");
    const more = page.getByRole("button", { name: "Show more", exact: true });
    await expect(items).toHaveCount(10);
    const first = await items.first().elementHandle();
    const width = await items
      .first()
      .locator(".bar-track > span")
      .getAttribute("style");
    const before = reads;
    const requestCount = requests.length;
    await more.focus();
    await page.keyboard.press("Enter");
    await expect(items).toHaveCount(20);
    await expect(more).toBeFocused();
    expect(
      await first?.evaluate(
        (element) => element === document.querySelector(".ranking-list li"),
      ),
    ).toBe(true);
    await expect(items.first().locator(".bar-track > span")).toHaveAttribute(
      "style",
      width ?? "",
    );
    await page.keyboard.press("Enter");
    await expect(items).toHaveCount(23);
    await expect(more).toHaveCount(0);
    await expect(
      page.getByRole("region", { name: `${label} rankings`, exact: true }),
    ).toBeFocused();
    for (let index = 0; index < 23; index += 1) {
      await expect(items.nth(index)).toContainText(`${name} ${index + 1}`);
      await expect(items.nth(index).locator(".rank")).toHaveText(
        String(index + 1),
      );
    }
    expect(reads).toBe(before);
    expect(requests.length).toBe(requestCount);
    if (category === "parties")
      await expect(list.getByRole("link")).toHaveCount(0);
  });
}

for (const count of [0, 7, 10, 20]) {
  test(`rankings exhaust correctly with ${count} entries`, async ({
    page,
    backend,
  }) => {
    await backend.route("**/api/rankings?*", async (route) =>
      route.fulfill({ json: completeRankingFixture(count) }),
    );
    await page.goto("/rankings");
    const items = page
      .getByRole("list", { name: "MPs rankings" })
      .getByRole("listitem");
    await expect(items).toHaveCount(Math.min(10, count));
    const more = page.getByRole("button", { name: "Show more" });
    if (count > 10) {
      await more.click();
      await expect(items).toHaveCount(count);
    }
    await expect(more).toHaveCount(0);
  });
}

test("revealed prefixes reset on committed category, history, period and refreshed snapshots", async ({
  page,
  backend,
}) => {
  await backend.route("**/api/rankings?*", async (route) =>
    route.fulfill({
      json: {
        ...completeRankingFixture(),
        period: new URL(route.request().url()).searchParams.get("period"),
      },
    }),
  );
  await page.goto("/rankings");
  const items = page.getByRole("listitem");
  const more = page.getByRole("button", { name: "Show more" });
  await more.click();
  await expect(items).toHaveCount(20);
  await page.getByRole("link", { name: "Companies", exact: true }).click();
  await expect(page).toHaveURL(/category=companies/);
  await expect(items).toHaveCount(10);
  await page.goBack();
  await expect(page).not.toHaveURL(/category=companies/);
  await expect(items).toHaveCount(10);
  await more.click();
  await page.goForward();
  await expect(page).toHaveURL(/category=companies/);
  await expect(items).toHaveCount(10);
  await more.click();
  await page
    .getByRole("combobox", { name: "Reporting period" })
    .selectOption("historical");
  await expect(page).toHaveURL(/period=historical/);
  await expect(items).toHaveCount(10);
  await more.click();
  await page.reload();
  await expect(items).toHaveCount(10);
});

test("pending period navigation keeps the committed prefix and disables reveal", async ({
  page,
  backend,
}) => {
  const delayed = Promise.withResolvers<void>();
  await backend.route("**/api/rankings?*", async (route) => {
    const period = new URL(route.request().url()).searchParams.get("period");
    if (period === "historical") await delayed.promise;
    await route.fulfill({ json: { ...completeRankingFixture(), period } });
  });
  await page.goto("/rankings");
  const more = page.getByRole("button", { name: "Show more" });
  await more.click();
  await page
    .getByRole("combobox", { name: "Reporting period" })
    .selectOption("historical");
  try {
    await expect(more).toBeDisabled();
    await expect(page.getByRole("listitem")).toHaveCount(20);
  } finally {
    delayed.resolve();
  }
  await expect(page).toHaveURL(/period=historical/);
  await expect(page.getByRole("listitem")).toHaveCount(10);
});

test("complete snapshots SSR renders only the initial prefix", async ({
  backend,
  request,
  baseURL,
}) => {
  await backend.route("**/api/rankings?*", async (route) =>
    route.fulfill({ json: completeRankingFixture() }),
  );
  const response = await request.get(`${baseURL}/rankings`);
  expect(response.ok()).toBe(true);
  const html = await response.text();
  expect(html).toContain('href="/members/10?period=current-term"');
  expect(html).not.toContain('href="/members/11?period=current-term"');
  expect(html).toContain("Show more");
});
