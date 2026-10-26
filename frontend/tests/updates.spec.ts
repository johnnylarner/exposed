import { expect, test } from "./backend";
import { updatesFixture } from "./updates-fixtures";

test("updates SSR lists all MPs and companies with profile links and UTC dates", async ({
  page,
  backend,
  request,
  baseURL,
}) => {
  await backend.route("**/api/updates", (route) =>
    route.fulfill({ json: updatesFixture }),
  );
  const response = await request.get(`${baseURL}/updates`);
  const html = await response.text();
  expect(html).toContain("Member 12");
  expect(html).toContain("Preferred company");
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
  await expect(
    page.getByRole("link", { name: "Preferred company" }),
  ).toHaveAttribute("href", "/funders/10000000-0000-0000-0000-000000000001");
  await page.setViewportSize({ width: 320, height: 900 });
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(
    320,
  );
  await expect(
    page.getByRole("heading", { name: "Companies", exact: true }),
  ).toBeVisible();
});

test("updates show unavailable and empty states", async ({ page, backend }) => {
  await backend.route("**/api/updates", (route) =>
    route.fulfill({ status: 503 }),
  );
  await page.goto("/updates");
  await expect(page.getByRole("alert")).toHaveText(
    "Updates couldn’t load. Try again.",
  );
  await backend.route("**/api/updates", (route) =>
    route.fulfill({ json: { ...updatesFixture, members: [], companies: [] } }),
  );
  await page.reload();
  await expect(page.getByText("No declarations found.")).toHaveCount(2);
});
