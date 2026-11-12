import { expect, test } from "./backend";
import { lordsFixture, sourceInterest } from "./lords-fixtures";

const lordEntity = {
  id: "2175",
  name: "Lord Example",
  kind: "Lords",
  funder_kind: null,
  match_source: { kind: "name" },
};

test("Lords SSR renders every occurrence in category columns with independent lifecycle states", async ({
  backend,
  page,
  request,
}) => {
  const calls: string[] = [];
  await backend.route("**/api/lords/2175", (route) => {
    calls.push(route.request().url());
    return route.fulfill({ json: lordsFixture });
  });
  const response = await request.get("/lords/2175?period=invalid");
  expect(response.status()).toBe(200);
  const html = await response.text();
  expect(html).toContain("Lord Example");
  expect(html).toContain("ARTIS (Europe) Ltd");
  expect(html).toContain("&lt;img");
  await page.goto("/lords/2175?q=Example&period=invalid");
  await expect(page.getByRole("heading", { level: 1 })).toHaveText(
    "Lord Example",
  );
  const left = page.getByRole("region", { name: "Shareholdings", exact: true });
  const right = page.getByRole("region", {
    name: "Registered interests",
    exact: true,
  });
  await expect(left.locator("h3")).toHaveCount(0);
  await expect(right.locator("h3")).toHaveText([
    "Category 888",
    "Category 5: Other interests",
    "Nil",
  ]);
  await expect(left.locator("summary")).toHaveText([
    "ARTIS (Europe) Ltd",
    "Boot (Henry) plc",
    "Repeated holding",
    "Repeated holding",
  ]);
  await expect(page.locator(".registered-interest")).toHaveCount(12);
  const compact = left.locator("details").first();
  await expect(compact).not.toHaveAttribute("open");
  await expect(compact.locator(".interest-text")).toBeHidden();
  await expect(
    left.getByText("Deleted 5 Apr 2025", { exact: true }),
  ).toBeVisible();
  await expect(left.getByText("Correction", { exact: true })).toBeVisible();
  await expect(left.locator("summary").nth(1)).toBeVisible();
  await compact.locator("summary").focus();
  await page.keyboard.press("Enter");
  await expect(compact).toHaveAttribute("open", "");
  await expect(compact.locator(".interest-text")).toHaveText(
    "ARTIS (Europe) Ltd (research and risk modelling)",
  );
  await expect(
    compact.getByText("Category 2: Shareholdings etc. (a)", { exact: true }),
  ).toBeVisible();
  await expect(
    compact.getByText("Created 14 Jun 2012", { exact: true }),
  ).toBeVisible();
  await expect(
    compact.getByText("Amended 1 Jan 2025", { exact: true }),
  ).toBeVisible();
  await page.keyboard.press("Space");
  await expect(compact).not.toHaveAttribute("open");
  await expect(left.locator("summary").nth(1)).toBeVisible();
  await expect(
    page.locator('.interest-content[data-deleted="true"]'),
  ).toHaveCount(3);
  await expect(page.locator(".interest-status")).toHaveText([
    "Deleted 5 Apr 2025",
    "Deleted 12 Nov 2026",
    "Deleted 2 Oct 2026",
  ]);
  const activeParent = page
    .locator(".interest-content")
    .filter({ has: page.getByText("Active parent", { exact: true }) });
  const deletedChild = page
    .locator(".interest-content")
    .filter({ has: page.getByText("Deleted child", { exact: true }) });
  const deletedParent = page
    .locator(".interest-content")
    .filter({ has: page.getByText("Deleted parent", { exact: true }) });
  const activeChild = page
    .locator(".interest-content")
    .filter({ has: page.getByText("Active child", { exact: true }) });
  await expect(activeParent).toHaveAttribute("data-deleted", "false");
  await expect(deletedChild).toHaveAttribute("data-deleted", "true");
  await expect(deletedParent).toHaveAttribute("data-deleted", "true");
  await expect(activeChild).toHaveAttribute("data-deleted", "false");
  await expect(activeChild.locator(".interest-text")).toHaveCSS(
    "color",
    "rgb(240, 237, 243)",
  );
  await expect(deletedParent.locator(".interest-text")).toHaveCSS(
    "color",
    "rgb(173, 166, 180)",
  );
  await expect(activeParent).toContainText("Correction");
  const metadata = activeParent.locator("details");
  await expect(metadata.locator("summary")).toHaveText("Details");
  await expect(metadata.locator(".interest-metadata")).toBeHidden();
  await metadata.locator("summary").click();
  await expect(
    metadata.getByText("Created 14 Jun 2012", { exact: true }),
  ).toBeVisible();
  await expect(deletedChild).not.toContainText("Correction");
  await expect(
    page
      .getByText("Interest ceased on 1 January 2026", { exact: true })
      .locator(".."),
  ).toHaveAttribute("data-deleted", "false");
  await expect(page.locator(".interest-correction")).toHaveCount(2);
  await expect(
    page.locator(".interest-text").filter({ hasText: "<img" }),
  ).toHaveText('<img src=x onerror="alert(1)"> & Partners\nSecond line');
  await expect(
    page.locator(".interest-text").filter({ hasText: "<img" }),
  ).toHaveCSS("white-space", "pre-wrap");
  await expect(page.locator("#lords img")).toHaveCount(0);
  await expect(right.locator(".interest-list").first()).toHaveCSS(
    "list-style-type",
    "disc",
  );
  await expect(activeParent).toHaveCSS("border-top-width", "0px");
  await expect(activeParent).toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
  await expect(
    page.getByText("Interest text unavailable", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("No registrable interests", { exact: true }),
  ).toBeVisible();
  await expect(page.getByRole("combobox")).toHaveCount(0);
  await expect(
    page.getByRole("link", { name: /history|summary/i }),
  ).toHaveCount(0);
  await expect(
    page.getByText(/Declared amounts|Historical|This parliamentary term/),
  ).toHaveCount(0);
  await expect(page.locator(".profile-type svg")).toHaveCSS(
    "color",
    "rgb(229, 138, 123)",
  );
  const back = page.getByRole("link", { name: "Back to search" });
  await expect(back).toHaveAttribute("href", "/search?q=Example");
  await back.focus();
  await expect(back).toHaveCSS("outline-color", "rgb(240, 240, 237)");
  const leftBounds = await left.boundingBox();
  const rightBounds = await right.boundingBox();
  expect(leftBounds).not.toBeNull();
  expect(rightBounds).not.toBeNull();
  if (leftBounds && rightBounds) {
    expect(leftBounds.x + leftBounds.width).toBeLessThan(rightBounds.x);
    expect(leftBounds.y).toBe(rightBounds.y);
    expect(rightBounds.width / leftBounds.width).toBeCloseTo(2, 1);
  }
  expect(calls.every((url) => url.endsWith("/api/lords/2175"))).toBe(true);
});

test("Lords search navigation uses warm red, its portcullis and the dedicated page", async ({
  backend,
  page,
}) => {
  await backend.route("**/api/search?*", (route) =>
    route.fulfill({ json: { entities: [lordEntity] } }),
  );
  await backend.route("**/api/lords/2175", (route) =>
    route.fulfill({ json: lordsFixture }),
  );
  await page.goto("/search?q=Example");
  const row = page.locator(".entity-row");
  await expect(row.locator(".entity-icon")).toHaveAccessibleName("Lords");
  await expect(row.locator(".entity-icon")).toHaveCSS(
    "color",
    "rgb(229, 138, 123)",
  );
  await expect(
    page.locator(".legend-item").filter({ hasText: "Lords" }).locator("svg"),
  ).toHaveCSS("color", "rgb(229, 138, 123)");
  const link = page.getByRole("link", { name: "Lord Example" });
  await expect(link).toHaveAttribute("href", "/lords/2175?q=Example");
  await link.click();
  await expect(page).toHaveURL(/\/lords\/2175\?q=Example$/u);
  await expect(
    page.getByRole("heading", { name: "Lord Example" }),
  ).toBeVisible();
  await page.getByRole("link", { name: "Back to search" }).click();
  await expect(page.getByRole("searchbox")).toHaveValue("Example");
});

test("compact occurrences retain every company, bounded source previews and exact source details across categories", async ({
  backend,
  page,
}) => {
  const multiple =
    "Project Alpha Topco Limited, Project Alpha Midco Limited and Project Alpha Bidco Limited (parent companies)";
  const narrative =
    "All shareholdings are managed by Rathbones and the member has no influence over any of the dealings although the following registrable shareholdings are held:";
  const unsafe = ' <img src=x onerror="alert(1)"> & Partners\nSecond line ';
  await backend.route("**/api/lords/2175", (route) =>
    route.fulfill({
      json: {
        ...lordsFixture,
        categories: [
          {
            ...lordsFixture.categories[1],
            interests: [
              { ...sourceInterest, text: multiple },
              { ...sourceInterest, text: narrative },
            ],
          },
          {
            ...lordsFixture.categories[1],
            source_id: 1003,
            name: "Category 2: Shareholdings etc. (b)",
            interests: [
              { ...sourceInterest, text: " \n " },
              { ...sourceInterest, text: unsafe },
            ],
          },
        ],
      },
    }),
  );
  await page.goto("/lords/2175");
  const left = page.getByRole("region", { name: "Shareholdings", exact: true });
  const summaries = left.locator("summary");
  await expect(left.locator("h3")).toHaveCount(0);
  await expect(left.locator(":scope > ul > .registered-interest")).toHaveCount(
    4,
  );
  await expect(summaries).toHaveText([
    "Project Alpha Topco Limited; Project Alpha Midco Limited; Project Alpha Bidco Limited",
    "All shareholdings are managed by Rathbones and the member has no influence over any of the dealings although…",
    "Interest text unavailable",
    '<img src=x onerror="alert(1)"> & Partners Second line',
  ]);
  await summaries.first().click();
  expect(await left.locator(".interest-text").first().textContent()).toBe(
    multiple,
  );
  await summaries.last().click();
  expect(await left.locator(".interest-text").last().textContent()).toBe(
    unsafe,
  );
  await expect(
    left
      .locator("details")
      .last()
      .getByText("Category 2: Shareholdings etc. (b)", { exact: true }),
  ).toBeVisible();
  await expect(left.locator("img")).toHaveCount(0);
});

for (const width of [360, 320]) {
  test(`Lords columns stack shareholdings first at ${width}px`, async ({
    backend,
    page,
  }) => {
    await backend.route("**/api/lords/2175", (route) =>
      route.fulfill({ json: lordsFixture }),
    );
    await page.setViewportSize({ width, height: 780 });
    await page.goto("/lords/2175");
    const left = await page
      .getByRole("region", { name: "Shareholdings", exact: true })
      .boundingBox();
    const right = await page
      .getByRole("region", { name: "Registered interests", exact: true })
      .boundingBox();
    expect(left).not.toBeNull();
    expect(right).not.toBeNull();
    if (left && right) {
      expect(left.x).toBe(right.x);
      expect(left.y + left.height).toBeLessThan(right.y);
    }
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= window.innerWidth,
      ),
    ).toBe(true);
    await expect(page.locator(".registered-interest")).toHaveCount(12);
  });
}

test("Lords has no history route and handles unavailable, missing and empty data", async ({
  backend,
  page,
}) => {
  let calls = 0;
  await backend.route("**/api/lords/2175", (route) => {
    calls += 1;
    return route.fulfill(
      calls === 1
        ? { status: 500 }
        : { json: { ...lordsFixture, categories: [] } },
    );
  });
  await page.goto("/lords/2175/history");
  await expect(
    page.getByRole("heading", { name: "Entity not found" }),
  ).toBeVisible();
  expect(calls).toBe(0);
  await page.goto("/lords/2175");
  await expect(page.getByRole("alert")).toContainText(
    "Registered interests couldn’t load",
  );
  await page.getByRole("button", { name: "Try again" }).click();
  await expect(
    page.getByRole("heading", { name: "Lord Example" }),
  ).toBeVisible();
  await expect(
    page.getByText("No interests supplied.", { exact: true }),
  ).toHaveCount(2);
  await backend.route("**/api/lords/2175", (route) =>
    route.fulfill({ status: 404 }),
  );
  await page.goto("/lords/2175");
  await expect(
    page.getByRole("heading", { name: "Member not found" }),
  ).toBeVisible();
});
