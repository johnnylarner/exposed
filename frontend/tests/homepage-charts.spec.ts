import { expect, test } from "./backend";
import { overviewFixture } from "./overview-fixture";

test("homepage charts render on the server and retain exact values", async ({
  browser,
  baseURL,
  backend,
}) => {
  await backend.route("**/api/overview", (route) =>
    route.fulfill({ json: overviewFixture() }),
  );
  const context = await browser.newContext({
    javaScriptEnabled: false,
    baseURL,
    viewport: { width: 1440, height: 1100 },
  });
  try {
    const page = await context.newPage();
    await page.goto("/");
    await expect(
      page.getByRole("heading", { name: "Distribution by quartile" }),
    ).toBeVisible();
    await expect(
      page.getByRole("img", { name: "Declared GBP ranges by rank quartile" }),
    ).toBeVisible();
    await expect(page.getByText("Alex Morgan", { exact: true })).toBeVisible();
    await expect(page.getByText("£150,000.00", { exact: true })).toBeVisible();
    await expect(
      page.getByText("8 current Commons MPs", { exact: false }),
    ).toBeVisible();
    await page.evaluate(() => document.fonts.ready);
    await page.screenshot({
      path: "/tmp/exposed-homepage-desktop.png",
      fullPage: true,
    });
    await page.setViewportSize({ width: 390, height: 844 });
    expect(
      await page.evaluate(() => document.documentElement.scrollWidth),
    ).toBe(390);
    await page.getByRole("link", { name: "Search MPs and funders" }).click();
    await expect(page.getByRole("searchbox")).toBeVisible();
  } finally {
    await context.close();
  }
});

test("negative and unavailable party totals stay visible", async ({
  page,
  backend,
}) => {
  const fixture = overviewFixture();
  fixture.parties[0].total.amount = "-12.34";
  await backend.route("**/api/overview", (route) =>
    route.fulfill({ json: fixture }),
  );
  await page.goto("/");
  await expect(
    page.getByText("Pie chart unavailable for negative declared amounts."),
  ).toBeVisible();
  await expect(page.getByText("Labour · −£12.34")).toBeVisible();
});
