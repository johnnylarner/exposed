import { expect, test } from "./backend";

test("about renders authored content on the server without declarations API", async ({
  page,
  request,
  backend,
}) => {
  const requests: string[] = [];
  await backend.route("**/api/*", async (route) => {
    requests.push(route.request().url());
    await route.fulfill({ status: 500 });
  });
  const response = await request.get("/about");
  expect(response.status()).toBe(200);
  expect(await response.text()).toContain("At Exposed we believe");
  await page.goto("/about", { waitUntil: "networkidle" });
  await expect(page).toHaveTitle("About · Exposed");
  await expect(
    page.getByRole("heading", { name: "About the project", level: 1 }),
  ).toBeVisible();
  await expect(page.getByRole("region", { name: "The Team" })).toBeVisible();
  await expect(
    page.getByRole("region", { name: "Product feedback" }),
  ).toBeVisible();
  const footer = page.getByRole("contentinfo");
  await expect(
    footer.getByRole("link", { name: "About", exact: true }),
  ).toHaveAttribute("aria-current", "page");
  await expect(footer.getByRole("link", { name: "Methodology" })).toHaveCount(
    0,
  );
  await page.keyboard.press("Tab");
  await page.keyboard.press("Enter");
  await expect(page.locator("#about")).toBeFocused();
  await expect(page.locator('article a[href=""]')).toHaveCount(0);
  expect(requests).toEqual([]);
});

test("about fits a narrow viewport and footer navigates back from methodology", async ({
  page,
}) => {
  await page.setViewportSize({ width: 320, height: 568 });
  await page.goto("/methodology");
  await page
    .getByRole("contentinfo")
    .getByRole("link", { name: "About", exact: true })
    .click();
  await expect(page).toHaveURL(/\/about$/u);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(
    320,
  );
});

test("feedback validates natively and retains input without JavaScript", async ({
  browser,
  application,
  request,
}) => {
  const context = await browser.newContext({
    javaScriptEnabled: false,
    extraHTTPHeaders: { "x-forwarded-proto": "http" },
  });
  const page = await context.newPage();
  try {
    await page.goto(`${application.url}/about`);
    await page.getByRole("button", { name: "Send feedback" }).click();
    await expect(page.getByLabel("Email", { exact: true })).toBeFocused();
    await page.getByLabel("Email", { exact: true }).fill("visitor@example.com");
    await page
      .getByLabel("Message", { exact: true })
      .fill("Please add more filters.");
    await page.getByRole("button", { name: "Send feedback" }).click();
    await expect(page.getByRole("alert")).toHaveText(
      "Feedback is temporarily unavailable. Please try again later.",
    );
    await expect(page.getByLabel("Email", { exact: true })).toHaveValue(
      "visitor@example.com",
    );
    await expect(page.getByLabel("Message", { exact: true })).toHaveValue(
      "Please add more filters.",
    );
    const invalid = await request.post("/about", {
      form: { email: "invalid", message: "" },
      headers: { origin: application.url, "x-forwarded-proto": "http" },
    });
    expect(invalid.status()).toBe(400);
    expect(await invalid.text()).toContain("Enter a valid email address.");
  } finally {
    await context.close();
  }
});
