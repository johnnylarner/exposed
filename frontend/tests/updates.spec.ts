import { expect, test } from "./backend";
import { updatesFixture } from "./updates-fixtures";

test("updates default to all MPs and switch locally with profile links and UTC dates", async ({
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
  expect(html).toContain("Member 12");

  await page.goto("/");
  await page.getByRole("link", { name: "Updates", exact: true }).click();
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("Updates");
  await expect(page.getByText("October 2026", { exact: true })).toBeVisible();
  const members = page.getByRole("list", { name: "MPs updates" });
  await expect(members.getByRole("listitem")).toHaveCount(12);
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
    updatesFixture.members.map(({ name }) => name),
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
  const companyRows = page.getByRole("list", { name: "Companies updates" });
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

test("updates show unavailable and empty states", async ({ page, backend }) => {
  await backend.route("**/api/updates", (route) =>
    route.fulfill({ status: 503 }),
  );
  await page.goto("/updates");
  await expect(page.getByRole("alert")).toHaveText(
    "Updates couldn’t load. Try again.",
  );
  await expect(
    page.getByRole("group", { name: "Updates category" }),
  ).toHaveCount(0);
  await backend.route("**/api/updates", (route) =>
    route.fulfill({ json: { ...updatesFixture, members: [] } }),
  );
  await page.reload();
  await expect(page.getByText("No declarations found.")).toHaveCount(1);
  await page.getByRole("button", { name: "Companies", exact: true }).click();
  await expect(
    page.getByRole("list", { name: "Companies updates" }),
  ).toBeVisible();
  await expect(page.getByText("No declarations found.")).toHaveCount(0);
  await backend.route("**/api/updates", (route) =>
    route.fulfill({ json: { ...updatesFixture, companies: [] } }),
  );
  await page.reload();
  await expect(page.getByRole("list", { name: "MPs updates" })).toBeVisible();
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
      page.getByRole("list", { name: `${category} updates` }),
    ).toBeVisible();
    expect(
      await page.evaluate(() => document.documentElement.scrollWidth),
    ).toBe(320);
    await expect(page.locator(".board")).toHaveCSS("animation-name", "none");
    await expect(page.locator(".board")).toHaveCSS("opacity", "1");
  }
});
