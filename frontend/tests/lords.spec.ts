import { expect, test } from "./backend";
import { lordsFixture, sourceInterest } from "./lords-fixtures";

const lordEntity = {
  id: "2175",
  name: "Lord Example",
  kind: "Lords",
  funder_kind: null,
  match_source: { kind: "name" },
};

test("Lords summary SSR shows nine active occurrences in the established columns without metadata or disclosures", async ({
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
  expect(html).toContain("Boot (Henry) plc");
  expect(html).toContain("&lt;img");
  expect(html.split("</main>")[0]).not.toContain("ARTIS (Europe) Ltd");
  await page.goto("/lords/2175?q=Example&period=invalid");
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
  await expect(left.locator(".interest-text")).toHaveText([
    "Boot (Henry) plc",
    "Repeated holding",
    "Repeated holding",
  ]);
  await expect(page.locator(".registered-interest")).toHaveCount(9);
  await expect(
    left.locator(".interest-content > .company-icon svg"),
  ).toHaveCount(3);
  await expect(right.locator(".company-icon")).toHaveCount(0);
  await expect(left.locator(".interest-text").first()).toHaveCSS(
    "font-size",
    "17px",
  );
  await expect(left.locator(".interest-text").first()).toHaveCSS(
    "font-weight",
    "600",
  );
  await expect(left.locator("svg").first()).toHaveCSS(
    "color",
    "rgb(173, 166, 180)",
  );
  await expect(right.locator("h3").first()).toHaveCSS(
    "color",
    "rgb(229, 138, 123)",
  );
  await expect(right.locator(".interest-text").first()).toHaveCSS(
    "font-size",
    "16px",
  );

  await expect(
    page.locator(
      "#lords details, #lords summary, .interest-status, .interest-correction, .interest-metadata",
    ),
  ).toHaveCount(0);
  await expect(page.getByText("Active child", { exact: true })).toBeVisible();
  await expect(
    page.getByText("Interest ceased on 1 January 2026", { exact: true }),
  ).toBeVisible();
  await expect(page.locator("#lords img")).toHaveCount(0);
  await expect(
    page.locator(".interest-text").filter({ hasText: "<img" }),
  ).toHaveText('<img src=x onerror="alert(1)"> & Partners\nSecond line');
  await expect(
    page.locator(".interest-text").filter({ hasText: "<img" }),
  ).toHaveCSS("white-space", "pre-wrap");
  await expect(
    page.getByText("Interest text unavailable", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("No registrable interests", { exact: true }),
  ).toBeVisible();
  await expect(page.getByRole("combobox")).toHaveCount(0);
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
    expect(rightBounds.width / leftBounds.width).toBeCloseTo(1, 1);
  }
  expect(calls.every((url) => url.endsWith("/api/lords/2175"))).toBe(true);
});

test("Lords history SSR retains all twelve occurrences, category ownership, full source text and independent lifecycle metadata", async ({
  backend,
  page,
  request,
}) => {
  await backend.route("**/api/lords/2175", (route) =>
    route.fulfill({ json: lordsFixture }),
  );
  const response = await request.get("/lords/2175/history?period=invalid");
  expect(response.status()).toBe(200);
  const html = await response.text();
  expect(html).toContain("ARTIS (Europe) Ltd (research and risk modelling)");
  expect(html).toContain("2025-01-01T01:00:00+02:00");
  expect(html).toContain("&lt;img");
  await page.goto("/lords/2175/history?q=Example&period=invalid");
  await expect(page.locator(".registered-interest")).toHaveCount(12);
  await expect(page.locator(".company-icon")).toHaveCount(0);
  await expect(page.locator(".interest-content").first()).toHaveCSS(
    "padding",
    "0px",
  );
  await expect(page.locator(".interest-content").first()).toHaveCSS(
    "border-bottom-width",
    "0px",
  );

  await expect(page.locator(".declaration")).toHaveCount(9);
  await expect(page.locator(".declaration h3")).toHaveText([
    "Category 888",
    "Category 888",
    "Category 2: Shareholdings etc. (a)",
    "Category 2: Shareholdings etc. (a)",
    "Category 2: Shareholdings etc. (a)",
    "Category 5: Other interests",
    "Category 5: Other interests",
    "Category 5: Other interests",
    "Nil",
  ]);
  await expect(page.locator(".declaration-heading > p")).toHaveText([
    "Category 888",
    "Category 888",
    "Category 1002",
    "Category 1002",
    "Category 1002",
    "Category 55",
    "Category 55",
    "Category 55",
    "Category 66",
  ]);
  await expect(
    page
      .locator(".interest-metadata")
      .getByText("Interest 10016", { exact: true }),
  ).toHaveCount(12);
  await expect(
    page.locator(".interest-text").filter({ hasText: /^Repeated holding$/u }),
  ).toHaveCount(2);
  await expect(page.locator(".interest-status")).toHaveText([
    "Deleted 5 Apr 2025",
    "Deleted 12 Nov 2026",
    "Deleted 2 Oct 2026",
  ]);
  await expect(page.locator(".interest-correction")).toHaveCount(2);
  for (const date of [
    "2012-06-14T12:49:14",
    "2025-01-01T01:00:00+02:00",
    "2026-11-12T23:45:00-09:00",
    "2026-10-02T00:00:00",
  ]) {
    const timestamp = page
      .locator(".interest-metadata time")
      .filter({ hasText: date })
      .first();
    await expect(timestamp).toBeVisible();
    await expect(timestamp).toHaveAttribute("datetime", date);
  }
  await expect(page.locator(".capture")).toHaveText(
    "Captured 2026-11-12T12:00:00Z",
  );
  await expect(page.locator("#lords details, #lords summary")).toHaveCount(0);
  await expect(
    page.getByRole("navigation", { name: "Interest history pages" }),
  ).toHaveCount(0);
  const activeParent = page
    .locator(".interest-content")
    .filter({ has: page.getByText("Active parent", { exact: true }) });
  const deletedParent = page
    .locator(".interest-content")
    .filter({ has: page.getByText("Deleted parent", { exact: true }) });
  const activeChild = page
    .getByText("Active child", { exact: true })
    .locator("..");
  const deletedChild = page
    .getByText("Deleted child", { exact: true })
    .locator("..");
  await expect(activeParent).toHaveAttribute("data-deleted", "false");
  await expect(deletedParent).toHaveAttribute("data-deleted", "true");
  await expect(activeChild).toHaveAttribute("data-deleted", "false");
  await expect(deletedChild).toHaveAttribute("data-deleted", "true");
  await expect(activeParent).toContainText("Correction");
  await expect(deletedChild).not.toContainText("Correction");
  await expect(activeChild.locator(".interest-text")).toHaveCSS(
    "color",
    "rgb(240, 237, 243)",
  );
  await expect(deletedParent.locator(".interest-text")).toHaveCSS(
    "color",
    "rgb(173, 166, 180)",
  );
  await expect(
    deletedParent.locator("..").locator(":scope > ul > li"),
  ).toHaveCount(1);
  await expect(page.locator("#lords img")).toHaveCount(0);
  await expect(
    page.locator(".interest-text").filter({ hasText: "<img" }),
  ).toHaveText('<img src=x onerror="alert(1)"> & Partners\nSecond line');
});

test("Lords views preserve query context and identity during same-lord navigation, and load another identity separately", async ({
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
  await page.getByRole("link", { name: "Lord Example" }).click();
  const summary = page.getByRole("link", { name: "Summary", exact: true });
  const history = page.getByRole("link", {
    name: "Interest history",
    exact: true,
  });
  await expect(summary).toHaveAttribute("aria-current", "page");
  await expect(history).toHaveAttribute(
    "href",
    "/lords/2175/history?q=Example",
  );
  let release: () => void = () => {};
  const pending = new Promise<void>((resolve) => {
    release = resolve;
  });
  await backend.route("**/api/lords/2175", async (route) => {
    await pending;
    await route.fulfill({ json: lordsFixture });
  });
  const called = backend.waitForRequest((request) =>
    request.url().endsWith("/api/lords/2175"),
  );
  await history.click();
  await called;
  try {
    await expect(page.locator("#lords")).toHaveAttribute("aria-busy", "true");
    await expect(
      page.getByRole("heading", { name: "Lord Example", exact: true }),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "Loading registered interests" }),
    ).toHaveCount(0);
    await expect(summary).toHaveAttribute("aria-current", "page");
  } finally {
    release();
  }
  await expect(page).toHaveURL(/\/lords\/2175\/history\?q=Example$/u);
  await expect(history).toHaveAttribute("aria-current", "page");
  await expect(summary).toHaveAttribute("href", "/lords/2175?q=Example");
  await summary.click();
  await expect(page.locator(".registered-interest")).toHaveCount(9);
  await page.goBack();
  await expect(history).toHaveAttribute("aria-current", "page");
  await page.getByRole("link", { name: "Back to search" }).click();
  await expect(page.getByRole("searchbox")).toHaveValue("Example");
  await page.getByRole("link", { name: "Lord Example" }).click();
  await expect(
    page.getByRole("heading", { name: "Lord Example", exact: true }),
  ).toBeVisible();
  let releaseOther: () => void = () => {};
  const other = new Promise<void>((resolve) => {
    releaseOther = resolve;
  });
  await backend.route("**/api/lords/2176", async (route) => {
    await other;
    await route.fulfill({
      json: {
        ...lordsFixture,
        member: { ...lordsFixture.member, id: "2176", name: "Another Lord" },
      },
    });
  });
  const otherCalled = backend.waitForRequest((request) =>
    request.url().endsWith("/api/lords/2176"),
  );
  await page.evaluate(() => {
    const link = document.createElement("a");
    link.href = "/lords/2176/history";
    document.querySelector("main")?.append(link);
    link.click();
  });
  await otherCalled;
  try {
    await expect(
      page.getByRole("heading", { name: "Loading registered interests" }),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "Lord Example", exact: true }),
    ).toHaveCount(0);
  } finally {
    releaseOther();
  }
  await expect(
    page.getByRole("heading", { name: "Another Lord", exact: true }),
  ).toBeVisible();
});

test("plain company labels remain compact while history retains exact blank, null, multiline and unsafe source strings", async ({
  backend,
  page,
}) => {
  const multiple =
    "Project Alpha Topco Limited, Project Alpha Midco Limited and Project Alpha Bidco Limited (parent companies)";
  const narrative =
    "All shareholdings are managed by Rathbones and the member has no influence over any of the dealings although the following registrable shareholdings are held:";
  const unsafe = ' <img src=x onerror="alert(1)"> & Partners\nSecond line ';
  const texts = [multiple, narrative, " \n ", "", null, unsafe];
  await backend.route("**/api/lords/2175", (route) =>
    route.fulfill({
      json: {
        ...lordsFixture,
        categories: [
          {
            ...lordsFixture.categories[1],
            interests: texts.map((text) => ({ ...sourceInterest, text })),
          },
        ],
      },
    }),
  );
  await page.goto("/lords/2175");
  await expect(page.locator(".interest-text")).toHaveText([
    "Project Alpha Topco Limited; Project Alpha Midco Limited; Project Alpha Bidco Limited",
    "All shareholdings are managed by Rathbones and the member has no influence over any of the dealings although…",
    "Interest text unavailable",
    "Interest text unavailable",
    "Interest text unavailable",
    '<img src=x onerror="alert(1)"> & Partners Second line',
  ]);
  await page
    .getByRole("link", { name: "Interest history", exact: true })
    .click();
  await expect(
    page.getByRole("link", { name: "Interest history", exact: true }),
  ).toHaveAttribute("aria-current", "page");
  expect(await page.locator(".interest-text").allTextContents()).toEqual(
    texts.map((text) => text ?? "Interest text unavailable"),
  );
  await expect(page.locator("#lords img, #lords details")).toHaveCount(0);
});

test("history paginates eleven whole roots and their children without ambiguous occurrence counts", async ({
  backend,
  page,
}) => {
  const interests = Array.from({ length: 11 }, (_, index) => ({
    ...sourceInterest,
    text: `Root ${index + 1}`,
    children: [
      {
        ...sourceInterest,
        text: `Child ${index + 1}`,
        children: [{ ...sourceInterest, text: `Grandchild ${index + 1}` }],
      },
    ],
  }));
  await backend.route("**/api/lords/2175", (route) =>
    route.fulfill({
      json: {
        ...lordsFixture,
        categories: [{ ...lordsFixture.categories[0], interests }],
      },
    }),
  );
  await page.goto("/lords/2175/history");
  const pagination = page.getByRole("navigation", {
    name: "Interest history pages",
  });
  await expect(pagination).toContainText("Page 1 of 2");
  await expect(page.locator(".declaration")).toHaveCount(10);
  await expect(page.locator(".registered-interest")).toHaveCount(30);
  await expect(
    pagination.getByRole("button", { name: "Previous" }),
  ).toBeDisabled();
  await expect(
    page.getByText(/Showing|most recent|Registered from|Registered to/),
  ).toHaveCount(0);
  await pagination.getByRole("button", { name: "Next" }).click();
  await expect(pagination).toContainText("Page 2 of 2");
  await expect(page.locator(".declaration")).toHaveCount(1);
  await expect(page.locator(".registered-interest")).toHaveCount(3);
  await expect(page.locator(".interest-text")).toHaveText([
    "Root 11",
    "Child 11",
    "Grandchild 11",
  ]);
  await expect(pagination.getByRole("button", { name: "Next" })).toBeDisabled();
  await pagination.getByRole("button", { name: "Previous" }).click();
  await expect(page.locator(".interest-text").first()).toHaveText("Root 1");
  await pagination.getByRole("button", { name: "Next" }).click();
  await page.getByRole("link", { name: "Summary", exact: true }).click();
  await page
    .getByRole("link", { name: "Interest history", exact: true })
    .click();
  await expect(pagination).toContainText("Page 1 of 2");
  await backend.route("**/api/lords/2176", (route) =>
    route.fulfill({
      json: {
        ...lordsFixture,
        member: { ...lordsFixture.member, id: "2176" },
        categories: [
          { ...lordsFixture.categories[0], interests: [interests[0]] },
        ],
      },
    }),
  );
  await pagination.getByRole("button", { name: "Next" }).click();
  await page.evaluate(() => {
    const link = document.createElement("a");
    link.href = "/lords/2176/history";
    document.querySelector("main")?.append(link);
    link.click();
  });
  await expect(page).toHaveURL(/\/lords\/2176\/history$/u);
  await expect(page.locator(".interest-text")).toHaveText([
    "Root 1",
    "Child 1",
    "Grandchild 1",
  ]);
  await expect(pagination).toHaveCount(0);
});

for (const width of [360, 320]) {
  for (const view of ["summary", "history"]) {
    test(`Lords ${view} has no overflow at ${width}px`, async ({
      backend,
      page,
    }) => {
      await backend.route("**/api/lords/2175", (route) =>
        route.fulfill({ json: lordsFixture }),
      );
      await page.setViewportSize({ width, height: 780 });
      await page.goto(`/lords/2175${view === "history" ? "/history" : ""}`);
      if (view === "summary") {
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
      }
      expect(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= window.innerWidth,
        ),
      ).toBe(true);
      await expect(page.locator(".registered-interest")).toHaveCount(
        view === "history" ? 12 : 9,
      );
    });
  }
}

for (const view of ["summary", "history"]) {
  test(`Lords ${view} handles errors, retry, missing members, absent and empty categories`, async ({
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
    const path = `/lords/2175${view === "history" ? "/history" : ""}`;
    await page.goto(path);
    await expect(page.getByRole("alert")).toContainText(
      "Registered interests couldn’t load",
    );
    await page.getByRole("button", { name: "Try again" }).click();
    await expect(
      page.getByRole("heading", { name: "Lord Example", exact: true }),
    ).toBeVisible();
    await expect(
      page.getByText("No interests supplied.", { exact: true }),
    ).toHaveCount(view === "history" ? 1 : 2);
    await backend.route("**/api/lords/2175", (route) =>
      route.fulfill({
        json: {
          ...lordsFixture,
          categories: [
            { ...lordsFixture.categories[0], interests: [] },
            {
              ...lordsFixture.categories[1],
              interests: [
                { ...sourceInterest, deleted_when: "2026-01-01T00:00:00" },
              ],
            },
          ],
        },
      }),
    );
    await page.reload();
    await expect(
      page.getByText("No interests supplied.", { exact: true }),
    ).toHaveCount(view === "history" ? 1 : 2);
    await expect(page.locator(".registered-interest")).toHaveCount(
      view === "history" ? 1 : 0,
    );
    await backend.route("**/api/lords/2175", (route) =>
      route.fulfill({ status: 404 }),
    );
    await page.reload();
    await expect(
      page.getByRole("heading", { name: "Member not found" }),
    ).toBeVisible();
  });
}
