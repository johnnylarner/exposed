import { expect, test } from "./backend";
import { sortUpdates } from "../src/lib/updates";
import { parseUpdates } from "../src/lib/server/updates";
import { updatesFixture, browseUpdatesFixture } from "./updates-fixtures";

test("updates default to ten MPs and switch locally with profile links and UTC dates", async ({
  page,
  backend,
  request,
  baseURL,
}) => {
  let requests = 0;
  await backend.route("**/api/updates", (route) => {
    requests += 1;
    return route.fulfill({ json: updatesFixture });
  });
  const response = await request.get(`${baseURL}/updates`);
  const html = await response.text();
  expect(html).toContain("Member 1");

  await page.goto("/");
  await page
    .getByRole("link", { name: "Latest declarations", exact: true })
    .click();
  await expect(page.getByRole("heading", { level: 1 })).toHaveText(
    "Latest declarations",
  );
  await expect(page).toHaveTitle("Latest declarations | Exposed");
  await expect(page.getByText("October 2026", { exact: true })).toBeVisible();
  const members = page.getByRole("list", { name: "MPs latest declarations" });
  await expect(members.getByRole("listitem")).toHaveCount(10);
  await expect(members.getByRole("link").first()).toHaveAttribute(
    "href",
    "/members/1",
  );
  await expect(members.getByRole("listitem").first()).toContainText(
    "2 declarations",
  );
  await expect(members.getByRole("listitem").first()).toContainText(
    "31 October 2026",
  );
  await expect(members.getByRole("link")).toHaveText(
    sortUpdates(parseUpdates(updatesFixture).members, "newest")
      .slice(0, 10)
      .map(({ name }) => name),
  );
  const mps = page.getByRole("button", { name: "MPs", exact: true });
  const companies = page.getByRole("button", {
    name: "Companies",
    exact: true,
  });
  await expect(mps).toHaveAttribute("aria-pressed", "true");
  await expect(companies).toHaveAttribute("aria-pressed", "false");
  const initialRequests = requests;
  await companies.click();
  await expect(companies).toBeFocused();
  await expect(companies).toHaveAttribute("aria-pressed", "true");
  await expect(mps).toHaveAttribute("aria-pressed", "false");
  const companyRows = page.getByRole("list", {
    name: "Companies latest declarations",
  });
  await expect(companyRows.getByRole("listitem")).toHaveCount(1);
  await expect(companyRows.getByRole("link")).toHaveAttribute(
    "href",
    "/funders/10000000-0000-0000-0000-000000000001",
  );
  await expect(companyRows).toContainText("3 declarations");
  await expect(companyRows).toContainText("31 October 2026");
  await mps.click();
  await expect(mps).toBeFocused();
  await expect(members).toBeVisible();
  await page.keyboard.press("Tab");
  await expect(companies).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(companyRows).toBeVisible();
  await expect(companies).toBeFocused();
  await page.keyboard.press("Shift+Tab");
  await expect(mps).toBeFocused();
  await page.keyboard.press("Space");
  await expect(members).toBeVisible();
  await expect(mps).toBeFocused();
  await expect(mps).toHaveAttribute("aria-pressed", "true");
  expect(requests).toBe(initialRequests);
});

test("updates horizontal halves select across their full scaled bounds", async ({
  page,
  backend,
}) => {
  await backend.route("**/api/updates", (route) =>
    route.fulfill({ json: updatesFixture }),
  );
  await page.goto("/updates");
  const wheel = page.getByRole("group", {
    name: "Latest declarations category",
  });
  const members = wheel.getByRole("button", { name: "MPs" });
  const companies = wheel.getByRole("button", { name: "Companies" });
  for (const width of [1280, 320]) {
    await page.setViewportSize({ width, height: 900 });
    await wheel.scrollIntoViewIfNeeded();
    const bounds = await wheel.boundingBox();
    if (!bounds) throw new Error("Missing updates wheel bounds");
    expect(bounds.width).toBe(220);
    const scale = bounds.width / 284;
    for (const x of [54, 230]) {
      await page.mouse.click(bounds.x + x * scale, bounds.y + 210 * scale);
      await expect(companies).toHaveAttribute("aria-pressed", "true");
      await page.mouse.click(bounds.x + 142 * scale, bounds.y + 132 * scale);
      await expect(companies).toHaveAttribute("aria-pressed", "true");
      await page.mouse.click(bounds.x + x * scale, bounds.y + 74 * scale);
      await expect(members).toHaveAttribute("aria-pressed", "true");
      await page.mouse.click(bounds.x + 142 * scale, bounds.y + 152 * scale);
      await expect(members).toHaveAttribute("aria-pressed", "true");
    }
    expect(
      await page.evaluate(() => document.documentElement.scrollWidth),
    ).toBe(width);
  }
});

test("updates show unavailable and empty states", async ({ page, backend }) => {
  await backend.route("**/api/updates", (route) =>
    route.fulfill({ status: 503 }),
  );
  await page.goto("/updates");
  await expect(page.getByRole("alert")).toHaveText(
    "Latest declarations couldn’t load. Try again.",
  );
  await expect(
    page.getByRole("group", { name: "Latest declarations category" }),
  ).toHaveCount(0);
  await backend.route("**/api/updates", (route) =>
    route.fulfill({ json: { ...updatesFixture, members: [] } }),
  );
  await page.reload();
  await expect(page.getByText("No declarations found.")).toHaveCount(1);
  await page.getByRole("button", { name: "Companies", exact: true }).click();
  await expect(
    page.getByRole("list", { name: "Companies latest declarations" }),
  ).toBeVisible();
  await expect(page.getByText("No declarations found.")).toHaveCount(0);
  await backend.route("**/api/updates", (route) =>
    route.fulfill({ json: { ...updatesFixture, companies: [] } }),
  );
  await page.reload();
  await expect(
    page.getByRole("list", { name: "MPs latest declarations" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Companies", exact: true }).click();
  await expect(page.getByText("No declarations found.")).toHaveCount(1);
});

test("updates fit narrow screens and respect reduced motion", async ({
  page,
  backend,
}) => {
  await backend.route("**/api/updates", (route) =>
    route.fulfill({ json: updatesFixture }),
  );
  await page.setViewportSize({ width: 320, height: 900 });
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/updates");
  for (const category of ["MPs", "Companies", "MPs"]) {
    await page.getByRole("button", { name: category, exact: true }).click();
    await expect(
      page.getByRole("list", { name: `${category} latest declarations` }),
    ).toBeVisible();
    expect(
      await page.evaluate(() => document.documentElement.scrollWidth),
    ).toBe(320);
    await expect(page.locator(".board")).toHaveCSS("animation-name", "none");
    await expect(page.locator(".board")).toHaveCSS("opacity", "1");
  }
});

test("updates browse sorted batches locally and restore focus after the last batch", async ({
  page,
  backend,
  browser,
  baseURL,
}) => {
  let requests = 0;
  await backend.route("**/api/updates", (route) => {
    requests += 1;
    return route.fulfill({ json: browseUpdatesFixture });
  });
  const ssr = await browser.newContext({ javaScriptEnabled: false });
  const ssrPage = await ssr.newPage();
  await ssrPage.goto(`${baseURL}/updates`);
  await expect(
    ssrPage
      .getByRole("list", { name: "MPs latest declarations" })
      .getByRole("link"),
  ).toHaveCount(10);
  await ssr.close();
  await page.goto("/updates");
  const initialRequests = requests;
  const links = page
    .getByRole("list", { name: "MPs latest declarations" })
    .getByRole("link");
  const newest = sortUpdates(
    parseUpdates(browseUpdatesFixture).members,
    "newest",
  );
  const oldest = sortUpdates(
    parseUpdates(browseUpdatesFixture).members,
    "oldest",
  );
  const more = page.getByRole("button", { name: "Show more" });
  const mps = page.getByRole("button", { name: "MPs", exact: true });
  const companies = page.getByRole("button", {
    name: "Companies",
    exact: true,
  });
  const oldestButton = page.getByRole("button", { name: "Oldest first" });
  const newestButton = page.getByRole("button", { name: "Newest first" });
  await expect(links).toHaveText(newest.slice(0, 10).map((row) => row.name));
  await page
    .locator(".board")
    .evaluate((board) => (board.dataset.retained = "yes"));
  await more.click();
  await expect(links).toHaveCount(20);
  await expect(more).toBeFocused();
  await expect(page.locator(".board")).toHaveAttribute("data-retained", "yes");
  await newestButton.click();
  await mps.click();
  await expect(links).toHaveCount(20);
  await more.click();
  await expect(links).toHaveText(newest.map((row) => row.name));
  await expect(more).toHaveCount(0);
  await expect(links.nth(20)).toBeFocused();
  await oldestButton.click();
  await expect(oldestButton).toHaveAttribute("aria-pressed", "true");
  await expect(links).toHaveText(oldest.slice(0, 10).map((row) => row.name));
  await more.click();
  await oldestButton.click();
  await expect(links).toHaveCount(20);
  await companies.click();
  await expect(oldestButton).toHaveAttribute("aria-pressed", "true");
  await expect(
    page
      .getByRole("list", { name: "Companies latest declarations" })
      .getByRole("link"),
  ).toHaveCount(10);
  await mps.click();
  await expect(links).toHaveText(oldest.slice(0, 10).map((row) => row.name));
  await newestButton.click();
  await expect(links).toHaveText(newest.slice(0, 10).map((row) => row.name));
  expect(requests).toBe(initialRequests);
});
