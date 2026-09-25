import { expect, test } from "@playwright/test";

test("live search preserves API ranking, spelling, duplicates and entity context", async ({
  page,
}) => {
  const entities = [
    { name: "john lewis partnership", kind: "Funder", funder_kind: "Company" },
    { name: "John Cooper", kind: "MP", funder_kind: null },
    { name: "John Kinder", kind: "Funder", funder_kind: "Individual" },
    { name: "john lewis partnership", kind: "Funder", funder_kind: "Company" },
    { name: "John Union", kind: "Funder", funder_kind: "Trade Union" },
    { name: "John Trust", kind: "Funder", funder_kind: "Not Specified" },
  ];
  const requests: URL[] = [];
  await page.route("**/api/search?*", async (route) => {
    requests.push(new URL(route.request().url()));
    await route.fulfill({ json: { entities } });
  });
  await page.goto("/");
  const input = page.getByRole("searchbox");
  await input.fill("jo");
  await expect(
    page.getByText("Type 1 more character to search."),
  ).toBeVisible();
  await input.press("Enter");
  expect(requests).toHaveLength(0);
  await input.fill("  John  ");
  await expect(
    page.getByRole("heading", { name: "6 results shown" }),
  ).toBeVisible();
  expect(requests).toHaveLength(1);
  expect(requests[0].searchParams.get("term")).toBe("John");
  expect(requests[0].searchParams.get("max_entries")).toBe("10");
  expect(requests[0].searchParams.get("strictness")).toBe("1");
  await expect(page.locator(".entity-name")).toHaveText(
    entities.map((entity) => entity.name),
  );
  await expect(page.locator(".entity-type")).toHaveText([
    "Funder: Company",
    "MP",
    "Funder: Individual",
    "Funder: Company",
    "Funder: Trade union",
    "Funder: Unclassified",
  ]);
  await expect(page.locator("mark").first()).toHaveText("john");
  await expect(page).toHaveURL(/q=John/);
});

test("a late response cannot replace a newer query or repopulate a cleared search", async ({
  page,
}) => {
  let releaseOld: () => void = () => {};
  let finishOld: () => void = () => {};
  const oldResponse = new Promise<void>((resolve) => {
    releaseOld = resolve;
  });
  const oldHandled = new Promise<void>((resolve) => {
    finishOld = resolve;
  });
  await page.route("**/api/search?*", async (route) => {
    const term = new URL(route.request().url()).searchParams.get("term");
    if (term === "John") await oldResponse;
    await route.fulfill({
      json: {
        entities: [
          {
            name: term === "John" ? "John Cooper" : "hsbc uk bank plc",
            kind: "Funder",
            funder_kind: "Company",
          },
        ],
      },
    });
    if (term === "John") finishOld();
  });
  await page.goto("/");
  const input = page.getByRole("searchbox");
  const oldRequest = page.waitForRequest((request) =>
    request.url().includes("term=John"),
  );
  await input.fill("John");
  await oldRequest;
  await input.fill("HSBC");
  await expect(page.locator(".entity-name")).toHaveText("hsbc uk bank plc");
  releaseOld();
  await oldHandled;
  await expect(page.locator(".entity-name")).toHaveText("hsbc uk bank plc");
  await expect(page.getByText("John Cooper", { exact: true })).toHaveCount(0);
  await page.getByRole("button", { name: "Clear search" }).click();
  await expect(input).toHaveValue("");
  await expect(
    page.getByRole("heading", { name: "Start with a name" }),
  ).toBeVisible();
  await expect(page.locator(".entity-row")).toHaveCount(0);
  await expect(input).toBeFocused();
  await expect(page).not.toHaveURL(/q=/);
});

test("an unavailable service offers retry and an empty response offers a useful next step", async ({
  page,
}) => {
  let calls = 0;
  await page.route("**/api/search?*", async (route) => {
    calls += 1;
    if (calls === 1)
      await route.fulfill({ status: 503, body: "service unavailable" });
    else await route.fulfill({ json: { entities: [] } });
  });
  await page.goto("/?q=unknown");
  await expect(page.getByRole("alert")).toContainText("Search couldn’t load");
  await page.getByRole("button", { name: "Try again" }).click();
  await expect(
    page.getByRole("heading", { name: "No matches for “unknown”" }),
  ).toBeVisible();
  await expect(
    page.getByText(
      "Try a surname, a shorter company name, or a different spelling.",
    ),
  ).toBeVisible();
  await page.getByRole("button", { name: "Edit your search" }).click();
  await expect(page.getByRole("searchbox")).toBeFocused();
  expect(calls).toBe(2);
});

test("search text and names containing markup or regular expression syntax remain literal", async ({
  page,
}) => {
  const name = 'A.* <img src=x onerror="alert(1)"> & Co';
  await page.route("**/api/search?*", (route) =>
    route.fulfill({
      json: { entities: [{ name, kind: "Funder", funder_kind: "Company" }] },
    }),
  );
  await page.goto("/");
  await page.getByRole("searchbox").fill("A.*");
  await expect(page.locator(".entity-name")).toHaveText(name);
  await expect(page.locator(".entity-name mark")).toHaveText("A.*");
  await expect(page.locator(".entity-name img")).toHaveCount(0);
});

test("keyboard search and long results remain usable on a narrow screen", async ({
  page,
}) => {
  await page.setViewportSize({ width: 360, height: 780 });
  await page.route("**/api/search?*", (route) =>
    route.fulfill({
      json: {
        entities: [
          {
            name: "Association of Research and Public Interest Organisations of the United Kingdom",
            kind: "Funder",
            funder_kind: "Unincorporated association",
          },
        ],
      },
    }),
  );
  await page.goto("/");
  await page.keyboard.press("/");
  await expect(page.getByRole("searchbox")).toBeFocused();
  await page.getByRole("searchbox").fill("Association");
  await expect(page.locator(".entity-type")).toHaveText(
    "Funder: Unincorporated association",
  );
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await page.keyboard.press("Escape");
  await expect(page.getByRole("searchbox")).toHaveValue("");
});
