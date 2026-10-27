import { expect, test } from "./backend";
import { rankingFixture } from "./rankings-fixtures";

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
  const individuals = page.getByRole("button", { name: "Individuals" });
  await individuals.focus();
  await page.keyboard.press("Space");

  await expect(individuals).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator(".category-wheel")).not.toContainText("✓");
  await expect(
    page.getByRole("list", { name: "Individuals rankings" }),
  ).toContainText("£0.00");
  await expect(
    page.getByRole("link", { name: "This parliamentary term", exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Companies" })
    .click({ position: { x: 55, y: 55 } });
  await expect(
    page.locator(".category-wheel path.sector-rim.selected"),
  ).toHaveCSS("stroke", "rgb(100, 207, 181)");
  await expect(
    page.getByRole("link", { name: "Example company" }),
  ).toHaveAttribute(
    "href",
    "/funders/10000000-0000-0000-0000-000000000002?period=current-term",
  );
  await expect(
    page.getByRole("list", { name: "Companies rankings" }),
  ).toContainText("−£12.3456789");
  await page.getByRole("button", { name: "Parties" }).click();
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
    route.fulfill({ json: rankingFixture }),
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
  await page.getByRole("button", { name: "Companies" }).click();
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
