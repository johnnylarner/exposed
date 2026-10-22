import { expect, test } from "./backend";

test("reference has shared navigation and needs no declarations API", async ({
  page,
  backend,
}) => {
  const requests: string[] = [];
  await backend.route("**/api/*", async (route) => {
    requests.push(route.request().url());
    await route.fulfill({ status: 500 });
  });
  await page.goto("/design-system");
  await expect(page).toHaveTitle("Design system · Exposed");
  await expect(page.getByRole("link", { name: "Exposed home" })).toBeVisible();
  await expect(
    page.getByRole("contentinfo").getByRole("link", { name: "Design system" }),
  ).toHaveAttribute("aria-current", "page");
  await page.keyboard.press("Tab");
  await page.keyboard.press("Enter");
  await expect(page.locator("#design-system")).toBeFocused();
  await page.getByRole("link", { name: "Read methodology" }).click();
  await expect(page).toHaveURL(/\/methodology$/u);
  expect(requests).toEqual([]);
});

test("fields retain associations and recover from reversed dates", async ({
  page,
}) => {
  await page.goto("/design-system");
  const query = page.getByRole("searchbox", { name: "Member or funder" });
  await expect(query).toHaveAccessibleDescription(
    "This example stays on this page and does not search parliamentary records. Try a name, such as Jane Smith.",
  );
  const example = page.getByRole("button", { name: "Use example name" });
  await example.focus();
  await page.keyboard.press("Space");
  await expect(query).toHaveValue("Jane Smith");
  await expect(page.getByRole("status")).toHaveText("No filters applied.");
  await expect(page.getByRole("button", { name: "Previous" })).toBeDisabled();
  await expect(page.getByLabel("Registration reference")).toBeDisabled();
  await page.getByLabel("Registered from", { exact: true }).fill("2025-02-01");
  const to = page.getByLabel("Registered to", { exact: true });
  await to.fill("2025-01-01");
  await page.getByRole("button", { name: "Apply filters" }).click();
  await expect(to).toHaveAttribute("aria-invalid", "true");
  await expect(to).toHaveAccessibleDescription(
    "Both dates are included. Choose an end date on or after the start date.",
  );
  await expect(page.getByRole("alert")).toBeVisible();
  await to.fill("2025-03-01");
  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.getByRole("button", { name: "Apply filters" }).click();
  await expect(page.getByRole("status")).toContainText(
    "Filters applied for Jane Smith",
  );
  await page.getByRole("button", { name: "Clear filters" }).click();
  await expect(query).toHaveValue("");
  await expect(to).toHaveValue("");
  await expect(page.getByRole("status")).toHaveText("No filters applied.");
});

test("funder actions inherit their focus color", async ({ page }) => {
  await page.goto("/design-system");
  const action = page.getByRole("link", { name: "Find a funder" });
  await action.focus();
  expect(
    await action.evaluate((element) => getComputedStyle(element).outlineColor),
  ).toBe("rgb(100, 207, 181)");
});

test("fields preserve their initial value through native form reset", async ({
  page,
}) => {
  await page.goto("/design-system");
  const name = page.getByRole("textbox", { name: "Funder name", exact: true });
  await expect(name).toHaveValue("Example Foundation");
  await name.fill("Another funder");
  await page.getByRole("button", { name: "Restore name" }).click();
  await expect(name).toHaveValue("Example Foundation");
});

test("reference fits a 320px viewport", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 568 });
  await page.goto("/design-system");
  for (const summary of await page.locator("summary").all())
    await summary.click();
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(
    320,
  );
});
