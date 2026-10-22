import { expect, test } from "./backend";

test.use({ development: true });

test("disclosures accept initial open state and native keyboard toggling", async ({
  page,
}) => {
  await page.goto("/design-system", { waitUntil: "networkidle" });
  const disclosure = page.getByRole("region", {
    name: "Recorded names example",
  });
  await expect(disclosure).toHaveAttribute("open", "");
  await disclosure.locator("summary").focus();
  await page.keyboard.press("Enter");
  await expect(disclosure).not.toHaveAttribute("open");
  await page.keyboard.press("Space");
  await expect(disclosure).toHaveAttribute("open", "");
});

test("reference has shared navigation and needs no declarations API", async ({
  page,
  backend,
}) => {
  const requests: string[] = [];
  await backend.route("**/api/*", async (route) => {
    requests.push(route.request().url());
    await route.fulfill({ status: 500 });
  });
  await page.goto("/design-system", { waitUntil: "networkidle" });
  await expect(page).toHaveTitle("Design system · Exposed");
  await expect(page.getByRole("heading", { level: 1 })).toHaveText(
    "Design system",
  );
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
  await page.goto("/design-system", { waitUntil: "networkidle" });
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

test("actions remain neutral across member and funder contexts", async ({
  page,
}) => {
  await page.goto("/design-system", { waitUntil: "networkidle" });
  for (const context of ["Member actions", "Funder actions"]) {
    const actions = page.locator(
      `[aria-label="${context}"] .ui-action:not(:disabled)`,
    );
    await expect(actions).toHaveCount(4);
    for (const action of await actions.all()) {
      await action.focus();
      await expect(action).toHaveCSS("outline-color", "rgb(240, 240, 237)");
      const variant = await action.getAttribute("data-variant");
      if (variant === "primary") {
        await expect(action).toHaveCSS(
          "background-color",
          "rgb(240, 240, 237)",
        );
        await expect(action).toHaveCSS("color", "rgb(17, 17, 17)");
      }
      if (variant === "quiet" || variant === "subtle")
        await expect(action).toHaveCSS("color", "rgb(240, 240, 237)");
      await action.hover();
      await expect(action).toHaveCSS("border-top-color", "rgb(240, 240, 237)");
      if (variant === "subtle")
        await expect(action).toHaveCSS("background-color", "rgb(40, 40, 40)");
    }
  }
});

test("fields preserve their initial value through native form reset", async ({
  page,
}) => {
  await page.goto("/design-system", { waitUntil: "networkidle" });
  const name = page.getByRole("textbox", { name: "Funder name", exact: true });
  await expect(name).toHaveValue("Example Foundation");
  await name.fill("Another funder");
  await page.getByRole("button", { name: "Restore name" }).click();
  await expect(name).toHaveValue("Example Foundation");
});

test("reference fits a 320px viewport", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 568 });
  await page.goto("/design-system", { waitUntil: "networkidle" });
  for (const summary of await page.locator("summary").all())
    await summary.click();
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(
    320,
  );
});

test("shared search preserves labels and disables composite clear actions", async ({
  page,
}) => {
  await page.goto("/design-system", { waitUntil: "networkidle" });
  const search = page.getByRole("searchbox", {
    name: "Example live search",
    exact: true,
  });
  await expect(search).toHaveAccessibleDescription(
    "Clear or edit this local example. No records are requested.",
  );
  await search.fill("Edited example");
  await page
    .locator(".search-field")
    .filter({ has: search })
    .getByRole("button", { name: "Clear search" })
    .click();
  await expect(search).toHaveValue("");
  for (const name of ["Disabled search", "Read only search"]) {
    const field = page.getByRole("searchbox", { name, exact: true });
    await expect(
      page
        .locator(".search-field")
        .filter({ has: field })
        .getByRole("button", { name: "Clear search" }),
    ).toBeDisabled();
  }
  await expect(
    page.getByRole("searchbox", { name: "Disabled search", exact: true }),
  ).toBeDisabled();
  await expect(
    page.getByRole("searchbox", { name: "Read only search", exact: true }),
  ).toHaveAttribute("readonly", "");
});

test("shared pagination and view links retain native navigation", async ({
  page,
}) => {
  await page.goto("/design-system", { waitUntil: "networkidle" });
  const pages = page.getByRole("navigation", { name: "Example pages" });
  await expect(pages.getByRole("button", { name: "Previous" })).toBeDisabled();
  await pages.getByRole("button", { name: "Next" }).click();
  await expect(pages).toContainText("Page 2 of 2");
  await expect(pages.getByRole("button", { name: "Next" })).toBeDisabled();
  await pages.getByRole("button", { name: "Previous" }).click();
  await expect(pages).toContainText("Page 1 of 2");
  const links = page.getByRole("navigation", { name: "Example views" });
  await expect(
    links.getByRole("link", { name: "Summary", exact: true }),
  ).toHaveAttribute("aria-current", "page");
  await links.getByRole("link", { name: "Declaration history" }).click();
  await expect(page).toHaveURL(/#data-patterns$/u);
});

test("repeated chart and indicator instances have distinct associations", async ({
  page,
}) => {
  await page.goto("/design-system", { waitUntil: "networkidle" });
  const charts = page.locator(".chart-example svg.earnings-chart");
  await expect(charts).toHaveCount(2);
  const associations = await charts.evaluateAll((elements) =>
    elements.map((element) =>
      element.getAttribute("aria-labelledby")!.split(" "),
    ),
  );
  expect(new Set(associations.flat()).size).toBe(4);
  for (const ids of associations)
    for (const id of ids)
      await expect(page.locator(`[id="${id}"]`)).toHaveCount(1);
  const tooltipIds: string[] = [];
  for (const chart of await page.locator(".chart-example").all()) {
    const entry = chart.locator(".funder-party-legend li");
    await entry.scrollIntoViewIfNeeded();
    await page.keyboard.press("Escape");
    await entry.dispatchEvent("pointerenter", { pointerType: "mouse" });
    const tooltip = page.getByRole("tooltip");
    await expect(tooltip).toHaveText(
      "Example party £90,071,992,547,409,993.123456789 2 entries, 1 amount unavailable",
    );
    tooltipIds.push((await tooltip.getAttribute("id"))!);
    await page.keyboard.press("Escape");
    await expect(tooltip).toHaveCount(0);
  }
  expect(new Set(tooltipIds).size).toBe(2);
  const indicatorIds: string[] = [];
  for (const label of ["Example MP", "Example funder"]) {
    const indicator = page.getByRole("button", { name: label, exact: true });
    await indicator.focus();
    const id = await indicator.getAttribute("aria-describedby");
    expect(id).toBeTruthy();
    indicatorIds.push(id!);
    await expect(page.locator(`[id="${id}"]`)).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(indicator).not.toHaveAttribute("aria-describedby");
  }
  expect(new Set(indicatorIds).size).toBe(2);
});

test("standalone metrics keep exact values keyboard scrollable", async ({
  page,
}) => {
  await page.setViewportSize({ width: 320, height: 568 });
  await page.goto("/design-system", { waitUntil: "networkidle" });
  const label = page.getByRole("heading", {
    name: "Known declared value",
    exact: true,
  });
  expect(
    await label.evaluate((element) => getComputedStyle(element).fontSize),
  ).toBe("12.8px");
  expect(
    await label.evaluate((element) => getComputedStyle(element).fontWeight),
  ).toBe("500");
  const total = page.getByRole("region", {
    name: "Known declared value",
    exact: true,
  });
  await expect(total).toHaveText("£90,071,992,547,409,993.123456789");
  await total.focus();
  await expect(total).toBeFocused();
  expect(
    await total.evaluate(
      (element) => element.scrollWidth > element.clientWidth,
    ),
  ).toBe(true);
  await page.keyboard.press("End");
  await expect(
    page.getByText("Amount unavailable", { exact: true }),
  ).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(
    320,
  );
});

test("appearance survives additive classes and independent heading ranks", async ({
  page,
}) => {
  await page.goto("/design-system", { waitUntil: "networkidle" });
  const result = page.getByText("Additive result badge", { exact: true });
  await expect(result).toHaveCSS("display", "inline-flex");
  await expect(result).toHaveCSS("border-top-width", "1px");
  await expect(page.getByText("Profile badge", { exact: true })).toHaveCSS(
    "font-size",
    "15px",
  );
  await expect(page.getByText("Legend badge", { exact: true })).toHaveCSS(
    "gap",
    "6px",
  );
  await expect(
    page.getByRole("heading", { name: "Centered semantic heading", level: 2 }),
  ).toHaveCSS("font-size", "16px");
  await expect(
    page.getByRole("heading", { name: "Detail semantic heading", level: 2 }),
  ).toHaveCSS("font-size", "38px");
  await expect(
    page.getByRole("heading", { name: "Portable compositions", level: 3 }),
  ).toHaveCSS("font-size", "22px");
  await expect(page.locator(".custom-state")).toHaveCSS("text-align", "center");
  await expect(page.locator(".custom-action.active-example")).toHaveCSS(
    "display",
    "inline-flex",
  );
  await expect(page.locator(".custom-link")).toHaveCSS(
    "display",
    "inline-flex",
  );
  await expect(page.locator(".custom-field")).toHaveCSS("display", "grid");
  await expect(page.locator(".custom-notice")).toHaveCSS(
    "border-left-width",
    "2px",
  );
  const disclosure = page.getByRole("region", {
    name: "Plain additive disclosure",
  });
  await disclosure.locator("summary").click();
  await expect(
    disclosure.getByText("Native disclosure content."),
  ).toBeVisible();
  await expect(disclosure).toHaveCSS("margin-bottom", "0px");
});

test("native text tables and standalone rows need no feature ancestors", async ({
  page,
}) => {
  await page.goto("/design-system", { waitUntil: "networkidle" });
  const table = page.getByRole("table", { name: "Three text columns" });
  for (const cell of await table.locator("th, td").all())
    await expect(cell).toHaveCSS("text-align", "left");
  const widths = await table
    .locator("thead th")
    .evaluateAll((cells) =>
      cells.map((cell) => cell.getBoundingClientRect().width),
    );
  expect(Math.max(...widths) - Math.min(...widths)).toBeLessThan(1);
  const list = page.getByRole("list", { name: "Standalone values" });
  await expect(list.locator(":scope > li")).toHaveCSS("display", "flex");
  await expect(list.getByText("Nested native item", { exact: true })).toHaveCSS(
    "display",
    "list-item",
  );
});

test("focus tokens propagate to native and component controls", async ({
  page,
}) => {
  await page.goto("/design-system", { waitUntil: "networkidle" });
  await page.locator("#design-system").evaluate((element) => {
    element.style.setProperty("--focus-width", "3px");
    element.style.setProperty("--focus-offset", "7px");
  });
  for (const [control, offset] of [
    [page.getByRole("link", { name: "Object class link" }), "7px"],
    [
      page
        .getByRole("navigation", { name: "Example views" })
        .getByRole("link", { name: "Summary", exact: true }),
      "8px",
    ],
    [
      page
        .locator(".search-field")
        .filter({
          has: page.getByRole("searchbox", {
            name: "Example live search",
            exact: true,
          }),
        })
        .getByRole("button", { name: "Clear search" }),
      "8px",
    ],
    [page.getByRole("button", { name: "Example MP", exact: true }), "8px"],
    [
      page.getByRole("region", { name: "Known declared value", exact: true }),
      "7px",
    ],
  ] as const) {
    await control.focus();
    await expect(control).toHaveCSS("outline-color", "rgb(240, 240, 237)");
    await expect(control).toHaveCSS("outline-width", "3px");
    await expect(control).toHaveCSS("outline-offset", offset);
  }
});
