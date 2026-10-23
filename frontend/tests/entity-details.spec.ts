import { expect, test, type Backend } from "./backend";

import {
  funderId,
  declaredTotals,
  emptyDeclaredTotals,
  member,
  memberHistoryFixture,
  memberDetails,
  funderDetails,
} from "./details-fixtures";

async function mockDetails(backend: Backend) {
  await backend.route("**/api/search?*", (route) =>
    route.fulfill({
      json: {
        entities: [
          {
            id: "1",
            name: member.name,
            kind: "MP",
            funder_kind: null,
            match_source: { kind: "name" },
          },
          {
            id: funderId,
            name: "Exact Funder",
            kind: "Funder",
            funder_kind: "Company",
            match_source: { kind: "name" },
          },
        ],
      },
    }),
  );
  await backend.route("**/api/members/*", (route) =>
    route.fulfill({ json: memberDetails }),
  );
  await backend.route("**/api/funders/*", (route) =>
    route.fulfill({ json: funderDetails }),
  );
}

test("native links preserve search through details, back, reload and new tabs", async ({
  backend,
  page,
  context,
}) => {
  await mockDetails(backend);
  await page.goto("/search?q=Example&strictness=0.42");
  const link = page.getByRole("link", { name: "Ava Example", exact: true });
  await expect(link).toHaveAttribute("href", "/members/1?q=Example");
  await expect(page.locator(".entity-name button")).toHaveCount(0);
  const popupPromise = context.waitForEvent("page");
  await link.click({ button: "middle" });
  const popup = await popupPromise;
  await expect(
    popup.getByRole("heading", { name: member.name, exact: true }),
  ).toBeVisible();
  await popup.close();
  await link.click();
  await expect(page).toHaveURL(/\/members\/1\?q=Example$/u);
  await expect(
    page.getByRole("heading", { name: "Recent declarations" }),
  ).toBeVisible();
  await page.reload();
  await expect(
    page.getByRole("heading", { name: member.name, exact: true }),
  ).toBeVisible();
  await page
    .getByRole("link", { name: "Exact Funder", exact: true })
    .first()
    .click();
  await expect(
    page.getByRole("heading", { name: "Funding by party" }).first(),
  ).toBeVisible();
  await expect(page).toHaveURL(
    new RegExp(`/funders/${funderId}\\?q=Example$`, "u"),
  );
  await page.getByRole("link", { name: "Ava Example", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: member.name, exact: true }),
  ).toBeVisible();
  await page.getByRole("link", { name: "Back to search", exact: true }).click();
  await expect(page.getByRole("searchbox")).toHaveValue("Example");
  await expect(page.getByRole("slider")).toHaveCount(0);
  await expect(page).toHaveURL(/\/search\?q=Example$/u);
  await link.click();
  await expect(
    page.getByRole("heading", { name: member.name, exact: true }),
  ).toBeVisible();
  await page.goBack();
  await expect(page.getByRole("searchbox")).toHaveValue("Example");
});

test("member summary shows recent declarations when none are donations", async ({
  backend,
  page,
}) => {
  await backend.route("**/api/members/1", (route) =>
    route.fulfill({
      json: {
        ...memberDetails,
        declarations: [
          {
            ...memberHistoryFixture.declarations[0],
            category_id: 1,
            category_name: "Employment",
          },
          memberHistoryFixture.declarations[1],
        ],
      },
    }),
  );
  await page.goto("/members/1");
  await expect(
    page.getByRole("heading", { name: "Recent declarations" }),
  ).toBeVisible();
  await expect(page.getByText("Declaration 120")).toBeVisible();
  await expect(page.getByText("Declaration 119")).toBeVisible();
  await expect(page.getByRole("heading", { name: "Employment" })).toBeVisible();
  await expect(
    page.getByRole("img", { name: "Monthly registered earnings" }),
  ).toBeVisible();
  await expect(
    page.getByText("View monthly amounts", { exact: true }),
  ).toHaveCount(0);
  await expect(page.locator(".earnings-chart rect title")).toContainText(
    "£18,014,398,509,481,986.246913578",
  );
  await expect(page.getByRole("heading", { name: "Visits" })).toBeVisible();
  await page.setViewportSize({ width: 360, height: 780 });
  await expect(
    page.getByRole("img", { name: "Monthly registered earnings" }),
  ).toBeVisible();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
});

test("member summary shows only the first five declarations in history order", async ({
  backend,
  page,
}) => {
  const declarations = Array.from({ length: 7 }, (_, index) => ({
    ...memberHistoryFixture.declarations[0],
    source_id: 120 - index,
    category_id: index === 0 ? 3 : 1,
    category_name: index === 0 ? "Support" : "Employment",
    entries: [],
  }));
  await backend.route("**/api/members/1", (route) =>
    route.fulfill({ json: { ...memberDetails, declarations } }),
  );
  await page.goto("/members/1");
  await expect(page.locator(".declaration-reference")).toHaveText([
    "Declaration 120",
    "Declaration 119",
    "Declaration 118",
    "Declaration 117",
    "Declaration 116",
  ]);
  await page.getByRole("link", { name: "View declaration history" }).click();
  await expect(page.locator(".declaration-reference")).toHaveCount(7);
});

test("member declarations retain duplicate precise amounts and missing source fields", async ({
  backend,
  page,
  context,
}) => {
  await mockDetails(backend);
  await page.goto("/members/1/history");
  await expect(page.locator(".profile-type")).toHaveText("MP");
  await expect(page.locator(".member-total")).toHaveCount(0);
  await expect(
    page.getByText("Showing 1 to 2 of 2 declarations"),
  ).toBeVisible();
  await expect(page.locator(".funding-value .amount")).toHaveText([
    "£9,007,199,254,740,993.123456789",
    "£9,007,199,254,740,993.123456789",
    "Amount unavailable",
  ]);
  for (const text of [
    "21 Jan 2026",
    "Registration date unavailable",
    "Funder unavailable",
    "Payment type unavailable",
    "The declaration included no monetary remuneration.",
  ])
    await expect(page.getByText(text, { exact: true })).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Top recipients" }),
  ).toHaveCount(0);
});

test("funder summary uses authoritative total, positive pie shares and reveals exact party totals", async ({
  backend,
  page,
  context,
}) => {
  await mockDetails(backend);
  await page.goto(`/funders/${funderId}?q=Example&strictness=0.42`);
  await expect(page.locator(".member-total")).toHaveText(
    "£18,014,398,509,481,981.246913578",
  );
  await expect(page.getByText("Company number 00123456")).toBeVisible();
  const controls = page.locator(".funder-party-legend li");
  await expect(page.getByRole("tooltip")).toHaveCount(0);
  await expect(page.locator(".funder-party-legend")).not.toContainText("£");
  await controls.first().hover();
  await expect(page.getByRole("tooltip")).toContainText(
    "£18,014,398,509,481,986.246913578",
  );
  await page.getByRole("tooltip").hover();
  await page.waitForTimeout(180);
  await expect(page.getByRole("tooltip")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("tooltip")).toHaveCount(0);
  await controls.nth(1).hover();
  await expect(page.getByRole("tooltip")).toContainText("−£5.00");
  await page.keyboard.press("Escape");
  await page.locator(".funder-party-chart svg g").hover();
  await expect(page.getByRole("tooltip")).toContainText(
    "£18,014,398,509,481,986.246913578",
  );
  await page.keyboard.press("Escape");
  await expect(
    page.getByRole("img", { name: "Monthly registered funding" }),
  ).toHaveCount(0);
  await expect(page.locator(".funder-party-chart svg circle")).toHaveCount(1);
  await expect(page.locator(".funder-recipients .amount")).toHaveText([
    "£18,014,398,509,481,986.246913578",
    "£0.00",
  ]);
  await expect(
    page.getByRole("heading", { name: "Top 5 recipients" }),
  ).toBeVisible();
  await expect(page.locator("main")).not.toContainText(
    /GBP|EUR|USD|currency|currencies/u,
  );
  await page.getByText("Recorded names", { exact: true }).click();
  await expect(page.getByText("EXACT FUNDER", { exact: true })).toBeVisible();
  await expect(
    page.getByRole("link", { name: "Funding history" }),
  ).toHaveAttribute("href", `/funders/${funderId}/history?q=Example`);
});

test("funding history retains sources, duplicate amounts and date paging", async ({
  backend,
  page,
}) => {
  const declarations = Array.from({ length: 12 }, (_, index) => ({
    member,
    declaration: {
      ...memberHistoryFixture.declarations[0],
      source_id: 120 - index,
      registered_at:
        index === 11
          ? null
          : `2026-01-${String(21 - index).padStart(2, "0")}T00:00:00Z`,
      entries: [
        ...memberHistoryFixture.declarations[0].entries.slice(0, 2),
        {
          funder: { id: funderId, name: "Exact Funder" },
          amount: "500",
          currency: "USD",
          payment_type: "Benefit",
        },
        {
          funder: { id: funderId, name: "Exact Funder" },
          amount: null,
          currency: null,
          payment_type: null,
        },
      ],
    },
  }));
  await backend.route("**/api/funders/*", (route) =>
    route.fulfill({ json: { ...funderDetails, declarations } }),
  );
  await page.goto(`/funders/${funderId}/history?q=Example&strictness=0.42`);
  await expect(page).toHaveURL(
    new RegExp(`/funders/${funderId}/history\\?q=Example$`, "u"),
  );
  await expect(page.locator(".member-total")).toHaveCount(0);
  await expect(page.locator(".declaration")).toHaveCount(10);
  await expect(
    page.locator(".declaration").first().locator(".amount"),
  ).toHaveText([
    "£9,007,199,254,740,993.123456789",
    "£9,007,199,254,740,993.123456789",
    "500",
    "Amount unavailable",
  ]);
  await expect(page.locator(".declaration").first()).toContainText("Support");
  await expect(page.locator(".declaration").first()).toContainText("Benefit");
  await expect(
    page
      .locator(".declaration")
      .first()
      .getByRole("link", { name: member.name }),
  ).toHaveAttribute("href", "/members/1?q=Example");
  await page.getByRole("button", { name: "Next", exact: true }).click();
  await expect(page.locator(".declaration")).toHaveCount(2);
  await expect(
    page.getByText("Registration date unavailable", { exact: true }),
  ).toBeVisible();
  await page.getByLabel("Registered from", { exact: true }).fill("2026-01-21");
  await expect(page.locator(".declaration")).toHaveCount(1);
  await expect(page.getByText("Page 1 of 1", { exact: true })).toBeVisible();
  await page.getByLabel("Registered to", { exact: true }).fill("2026-01-20");
  await expect(page.getByRole("alert")).toContainText("start date");
  await page.getByRole("button", { name: "Clear dates" }).click();
  await expect(page.locator(".declaration")).toHaveCount(10);
  await expect(page.locator("main")).not.toContainText(
    /GBP|EUR|USD|currency|currencies/u,
  );
  await page.setViewportSize({ width: 360, height: 780 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await page.getByRole("link", { name: "Summary", exact: true }).click();
  await expect(page).toHaveURL(
    new RegExp(`/funders/${funderId}\\?q=Example$`, "u"),
  );
});

test("former MPs retain their latest membership description without a constituency label", async ({
  backend,
  page,
}) => {
  await backend.route("**/api/members/1", (route) =>
    route.fulfill({
      json: {
        ...memberDetails,
        member: {
          ...member,
          membership_from: "Life peer",
          is_current_commons: false,
        },
      },
    }),
  );
  await page.goto("/members/1");
  await expect(page.getByText("Life peer", { exact: true })).toBeVisible();
  await expect(page.getByText("Former MP", { exact: true })).toBeVisible();
  await expect(page.getByText("Constituency", { exact: true })).toHaveCount(0);
});

test("errors allow retry and missing or invalid routes offer search navigation", async ({
  backend,
  page,
}) => {
  let calls = 0;
  await backend.route("**/api/members/*", async (route) => {
    calls += 1;
    await route.fulfill(
      calls === 1
        ? { status: 503, body: "private database error" }
        : { json: memberDetails },
    );
  });
  await page.goto("/members/1?q=Example&strictness=0.4");
  await expect(page.getByRole("alert")).toContainText("Details couldn’t load");
  await expect(page.getByText("private database error")).toHaveCount(0);
  await page.getByRole("button", { name: "Try again" }).click();
  await expect(
    page.getByRole("heading", { name: member.name, exact: true }),
  ).toBeVisible();
  await backend.route("**/api/members/999", (route) =>
    route.fulfill({ status: 404 }),
  );
  await page.goto("/members/999?q=Example");
  await expect(
    page.getByRole("heading", { name: "Entity not found" }),
  ).toBeVisible();
  await expect(
    page.getByRole("link", { name: "Search MPs and funders", exact: true }),
  ).toHaveAttribute("href", "/search?q=Example");
  await page.goto("/members/0");
  await expect(
    page.getByRole("heading", { name: "Entity not found" }),
  ).toBeVisible();
  expect(calls).toBe(2);
});

test("empty details and unknown currency only details remain distinct from errors", async ({
  backend,
  page,
}) => {
  await backend.route("**/api/members/1", (route) =>
    route.fulfill({
      json: {
        ...memberDetails,
        declarations: [],
        summary: {
          ...memberDetails.summary,
          declared_totals: emptyDeclaredTotals,
          currencies: [],
          first_registered_at: null,
          last_registered_at: null,
          undated_declaration_count: 0,
          entry_count: 0,
          unknown_currency_count: 0,
        },
      },
    }),
  );
  await backend.route("**/api/funders/*", (route) =>
    route.fulfill({
      json: {
        ...funderDetails,
        currencies: [],
        entry_count: 0,
        unknown_currency_count: 0,
      },
    }),
  );
  await page.goto("/members/1");
  await expect(
    page.getByText("No known declared values available."),
  ).toBeVisible();
  await expect(
    page.getByText("No declarations are stored for this MP."),
  ).toBeVisible();
  await expect(page.locator(".declared-value")).toHaveText([
    "No declaration data available",
    "No declaration data available",
    "No declaration data available",
  ]);
  await expect(
    page.locator(".member-declared-totals th[scope=row]"),
  ).toHaveText(["Total declared", "Declared this term", "Declared in 2026"]);
  await page.goto(`/funders/${funderId}`);
  await expect(
    page.getByText("No funding entries are stored for this funder."),
  ).toBeVisible();
  await backend.route("**/api/funders/*", (route) =>
    route.fulfill({
      json: {
        ...funderDetails,
        currencies: [],
        entry_count: 2,
        unknown_currency_count: 2,
      },
    }),
  );
  await page.reload();
  await expect(
    page.getByText("No known funding totals are available."),
  ).toBeVisible();
});

test("long names and exact amounts fit mobile and source markup stays literal", async ({
  backend,
  page,
}) => {
  const longName =
    'Association <img src=x onerror="alert(1)"> for Research and Public Interest Organisations';
  await page.setViewportSize({ width: 360, height: 780 });
  await backend.route("**/api/funders/*", (route) =>
    route.fulfill({
      json: {
        ...funderDetails,
        funder: {
          ...funderDetails.funder,
          name: longName,
          aliases: [longName, "A".repeat(500)],
        },
        currencies: funderDetails.currencies.map((group) => ({
          ...group,
          parties: group.parties.map((party) => ({
            ...party,
            party_name:
              "The Party of Research and Public Interest Organisations",
          })),
        })),
      },
    }),
  );
  await page.goto(`/funders/${funderId}`);
  await expect(
    page.getByRole("heading", { name: longName, exact: true }),
  ).toBeVisible();
  await page.getByText("Recorded names", { exact: true }).click();
  const recordedNames = page.getByRole("region", { name: "Recorded names" });
  await expect(recordedNames.getByRole("listitem")).toHaveText([
    longName,
    "A".repeat(500),
  ]);
  await expect(page.locator("#details img")).toHaveCount(0);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await backend.route("**/api/members/*", (route) =>
    route.fulfill({
      json: { ...memberDetails, member: { ...member, name: longName } },
    }),
  );
  await page.goto("/members/1");
  await expect(
    page.getByRole("heading", { name: longName, exact: true }),
  ).toBeVisible();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await page.keyboard.press("/");
  await expect(page.getByRole("searchbox")).toHaveCount(0);
});

test("detail timeout offers retry and leaving a loading page cancels its result", async ({
  backend,
  page,
}) => {
  let release: () => void = () => {};
  const pending = new Promise<void>((resolve) => {
    release = resolve;
  });
  await backend.route("**/api/members/*", async (route) => {
    await pending;
    await route.fulfill({ json: memberDetails }).catch(() => {});
  });
  await page.goto("/members/1?q=Example");
  await expect(page.getByRole("alert")).toContainText(
    "Details are taking too long",
  );
  await page.getByRole("button", { name: "Try again" }).click();
  await expect(
    page.getByRole("heading", { name: "Loading details" }),
  ).toBeVisible();
  await backend.route("**/api/search?*", (route) =>
    route.fulfill({ json: { entities: [] } }),
  );
  await page.getByRole("link", { name: "Back to search", exact: true }).click();
  release();
  await expect(page.getByRole("searchbox")).toHaveValue("Example");
  await expect(
    page.getByRole("heading", { name: member.name, exact: true }),
  ).toHaveCount(0);
});

test("recorded names retain stored variants for every funder kind and hide empty lists", async ({
  backend,
  page,
}) => {
  for (const kind of ["Company", "Individual", null]) {
    await backend.route("**/api/funders/*", (route) =>
      route.fulfill({
        json: {
          ...funderDetails,
          funder: { ...funderDetails.funder, funder_kind: kind },
        },
      }),
    );
    await page.goto(`/funders/${funderId}`);
    await page.getByText("Recorded names", { exact: true }).click();
    const names = page
      .getByRole("region", { name: "Recorded names" })
      .getByRole("listitem");
    await expect(names).toHaveCount(funderDetails.funder.aliases.length);
    expect(await names.allTextContents()).toEqual(funderDetails.funder.aliases);
  }
  await backend.route("**/api/funders/*", (route) =>
    route.fulfill({
      json: {
        ...funderDetails,
        funder: { ...funderDetails.funder, aliases: [] },
      },
    }),
  );
  await page.reload();
  await expect(
    page.getByRole("heading", { name: "Exact Funder", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("region", { name: "Recorded names" }),
  ).toHaveCount(0);
});

for (const negative of [false, true]) {
  test(`category bars show full-track shares with negative amounts ${negative}`, async ({
    backend,
    page,
  }) => {
    await backend.route("**/api/members/1", (route) =>
      route.fulfill({
        json: {
          ...memberDetails,
          summary: {
            ...memberDetails.summary,
            currencies: [
              {
                ...memberDetails.summary.currencies[0],
                amount: negative ? "60" : "100",
                categories: [
                  {
                    category_id: 1,
                    category_name: "Employment",
                    amount: "80",
                    entry_count: 1,
                    unknown_amount_count: 0,
                  },
                  {
                    category_id: 2,
                    category_name: "Gifts",
                    amount: negative ? "-20" : "20",
                    entry_count: 1,
                    unknown_amount_count: 0,
                  },
                ],
              },
            ],
          },
        },
      }),
    );
    await page.goto("/members/1");
    await expect(page.locator(".signed-bar > span")).toHaveCount(2);
    const shares = await page.locator(".signed-bar").evaluateAll((tracks) =>
      tracks.map((track) => {
        const bar = track.querySelector("span");
        return bar
          ? bar.getBoundingClientRect().width /
              track.getBoundingClientRect().width
          : 0;
      }),
    );
    const positions = await page.locator(".signed-bar").evaluateAll((tracks) =>
      tracks.map((track) => {
        const bar = track.querySelector("span");
        const bounds = track.getBoundingClientRect();
        return bar
          ? (bar.getBoundingClientRect().left - bounds.left) / bounds.width
          : 0;
      }),
    );
    expect(positions[0]).toBeCloseTo(negative ? 0.2 : 0, 3);
    expect(positions[1]).toBeCloseTo(0, 3);
    expect(shares[0]).toBeCloseTo(0.8, 3);
    expect(shares[1]).toBeCloseTo(0.2, 3);
    expect(shares.reduce((total, share) => total + share, 0)).toBeCloseTo(1, 3);
  });
}

test("member summary retains signed category bars alongside all-category totals", async ({
  backend,
  page,
  context,
}) => {
  await mockDetails(backend);
  await page.goto("/members/1");
  await expect(page.locator(".declared-value")).toHaveText([
    "£18,014,398,509,481,986.246913578",
    "£18,014,398,509,481,986.246913578",
    "£18,014,398,509,481,986.246913578",
  ]);
  await expect(page.locator(".signed-bar .negative")).toHaveCount(1);
  await expect(
    page
      .locator(".category-label")
      .filter({ hasText: "Unrecognized source category" }),
  ).toContainText("−£5.00");
  await expect(
    page.locator(
      ".member-currency-totals, .member-overview, .category-currency",
    ),
  ).toHaveCount(0);
  await expect(page.locator(".member-identity")).toContainText(
    "Declared this term",
  );
  const total = await page.locator(".declared-value").first().boundingBox();
  const tabs = await page.locator(".member-tabs").boundingBox();
  expect(total && tabs && total.y + total.height <= tabs.y).toBeTruthy();
  await expect(page.locator(".declarations-list > li")).toHaveCount(2);
  await expect(
    page.getByRole("link", { name: "View declaration history" }),
  ).toHaveAttribute("href", "/members/1/history");
});

test("history filters the full dataset before paging and resets page for date changes", async ({
  backend,
  page,
}) => {
  const declarations = Array.from({ length: 25 }, (_, index) => ({
    source_id: 125 - index,
    category_id: index === 24 ? 987 : 1,
    category_name: index === 24 ? "Unknown source category" : "Employment",
    registered_at:
      index === 24
        ? null
        : `2026-01-${String(25 - index).padStart(2, "0")}T23:59:59Z`,
    entries: [],
  }));
  await backend.route("**/api/members/1", (route) =>
    route.fulfill({ json: { ...memberDetails, declarations } }),
  );
  await page.goto("/members/1/history?q=Example&strictness=0.42");
  await expect(page).toHaveURL(/members\/1\/history\?q=Example$/u);
  await expect(page.locator(".declaration-reference")).toHaveText(
    Array.from({ length: 10 }, (_, index) => `Declaration ${125 - index}`),
  );
  await page.getByRole("button", { name: "Next", exact: true }).click();
  await expect(page.getByText("Page 2 of 3", { exact: true })).toBeVisible();
  await page.getByLabel("Registered to").fill("2026-01-08");
  await expect(page.getByText("Page 1 of 1", { exact: true })).toBeVisible();
  await expect(page.locator(".declaration-reference")).toHaveText(
    Array.from({ length: 7 }, (_, index) => `Declaration ${108 - index}`),
  );
  await page.getByLabel("Registered from").fill("2026-01-08");
  await expect(page.locator(".declaration-reference")).toHaveText([
    "Declaration 108",
  ]);
  await expect(
    page.getByText("Declarations without a registration date are excluded.", {
      exact: false,
    }),
  ).toBeVisible();
  await page.getByLabel("Registered from").fill("2026-01-09");
  await expect(page.getByRole("alert")).toContainText(
    "start date must be on or before",
  );
  await expect(page.locator(".declarations-list")).toHaveCount(0);
  await page.getByRole("button", { name: "Clear dates" }).click();
  await expect(page.getByText("Page 1 of 3", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Next", exact: true }).click();
  await page.getByRole("button", { name: "Next", exact: true }).click();
  await expect(
    page.getByText("Registration date unavailable", { exact: true }),
  ).toBeVisible();
  await page.getByLabel("Registered from").fill("2027-01-01");
  await expect(
    page.getByText(
      "No declarations match these dates. Clear or change the dates.",
    ),
  ).toBeVisible();
  await page.getByRole("button", { name: "Clear dates" }).click();
  await page.getByRole("link", { name: "Summary", exact: true }).click();
  await expect(page).toHaveURL(/members\/1\?q=Example$/u);
  await page.goBack();
  await expect(
    page.getByRole("heading", { name: "Declaration history", exact: true }),
  ).toBeVisible();
  await page.reload();
  await expect(page.getByLabel("Registered from")).toHaveValue("");
  await page.setViewportSize({ width: 360, height: 780 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
});

test("party pie keeps zero and unavailable totals and explains unavailable shares", async ({
  backend,
  page,
}) => {
  await backend.route("**/api/funders/*", (route) =>
    route.fulfill({
      json: {
        ...funderDetails,
        currencies: [
          {
            ...funderDetails.currencies[1],
            amount: "0",
            parties: [
              {
                party_id: 1,
                party_name: "Zero Party",
                amount: "0",
                entry_count: 1,
                unknown_amount_count: 0,
              },
              {
                party_id: 2,
                party_name: "Unknown Party",
                amount: null,
                entry_count: 1,
                unknown_amount_count: 1,
              },
            ],
          },
        ],
      },
    }),
  );
  await page.goto(`/funders/${funderId}`);
  await expect(
    page.getByText("Pie unavailable. No positive known totals are recorded."),
  ).toBeVisible();
  await page.locator(".funder-party-legend li").first().hover();
  await expect(page.getByRole("tooltip")).toContainText("£0.00");
  await page.locator(".funder-party-legend li").nth(1).hover();
  await expect(page.getByRole("tooltip")).toContainText("Amount unavailable");
  await expect(page.locator(".funder-party-chart path")).toHaveCount(0);
});

for (const amounts of [
  ["3", "7"],
  ["0", "-5"],
  [null, "0"],
]) {
  test(`party legend exposes totals for ${amounts.join("/")}`, async ({
    backend,
    page,
  }) => {
    const currencies = funderDetails.currencies.map((group) =>
      group.currency !== "GBP"
        ? group
        : {
            ...group,
            parties: group.parties.map((party, index) => ({
              ...party,
              amount: amounts[index],
            })),
          },
    );
    await backend.route("**/api/funders/*", (route) =>
      route.fulfill({ json: { ...funderDetails, currencies } }),
    );
    await page.goto(`/funders/${funderId}`);
    await expect(page.locator(".funder-party-legend")).not.toContainText("£");
    for (const [index, amount] of amounts.entries()) {
      const total =
        amount === null
          ? "Amount unavailable"
          : amount === "-5"
            ? "−£5.00"
            : `£${amount}.00`;
      const item = page.locator(".funder-party-legend li").nth(index);
      await expect(item).toHaveAccessibleName(
        `Party ${index === 0 ? "A" : "B"}: ${total}`,
      );
      await item.hover();
      await expect(page.getByRole("tooltip")).toContainText(total);
      await page.keyboard.press("Escape");
    }
    if (amounts[0] === "3") {
      await expect(page.locator(".funder-party-chart svg path")).toHaveCount(2);
      await page.locator(".funder-party-chart svg g").nth(1).hover();
      await expect(page.getByRole("tooltip")).toContainText("£7.00");
    }
    await page.setViewportSize({ width: 800, height: 900 });
    const left = await page
      .locator(".funder-summary-top > section")
      .first()
      .boundingBox();
    const right = await page
      .locator(".funder-summary-top > section")
      .nth(1)
      .boundingBox();
    expect(right?.y).toBeGreaterThan((left?.y ?? 0) + (left?.height ?? 0));
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= window.innerWidth,
      ),
    ).toBe(true);
  });
}

test("party tooltip persists when the pointer crosses between pie slices", async ({
  backend,
  page,
}) => {
  const currencies = funderDetails.currencies.map((group) =>
    group.currency !== "GBP"
      ? group
      : {
          ...group,
          amount: "10",
          parties: group.parties.map((party) => ({ ...party, amount: "5" })),
        },
  );
  await backend.route("**/api/funders/*", (route) =>
    route.fulfill({ json: { ...funderDetails, currencies } }),
  );
  await page.goto(`/funders/${funderId}`);
  const pie = page.locator(".funder-party-chart svg");
  await expect(pie.locator("path")).toHaveCount(2);
  const bounds = await pie.evaluate((element) => {
    const { x, y, width, height } = element.getBoundingClientRect();
    return { x, y, width, height };
  });
  await page.mouse.move(
    bounds.x + bounds.width * 0.7,
    bounds.y + bounds.height * 0.5,
  );
  await expect(page.getByRole("tooltip")).toContainText("Party A");
  await page.mouse.move(
    bounds.x + bounds.width * 0.3,
    bounds.y + bounds.height * 0.5,
  );
  await expect(page.getByRole("tooltip")).toContainText("Party B");
  await page.waitForTimeout(250);
  await expect(page.getByRole("tooltip")).toContainText("Party B");
});

test("small pie slice tooltips do not cover neighboring slices", async ({
  backend,
  page,
}) => {
  const currencies = funderDetails.currencies.map((group) =>
    group.currency !== "GBP"
      ? group
      : {
          ...group,
          amount: "10",
          parties: group.parties.map((party, index) => ({
            ...party,
            amount: index === 0 ? "1" : "9",
          })),
        },
  );
  await backend.route("**/api/funders/*", (route) =>
    route.fulfill({ json: { ...funderDetails, currencies } }),
  );
  await page.goto(`/funders/${funderId}`);
  const pie = page.locator(".funder-party-chart svg");
  await expect(pie.locator("path")).toHaveCount(2);
  const bounds = await pie.evaluate((element) => {
    const { x, y, width, height, top, bottom } =
      element.getBoundingClientRect();
    return { x, y, width, height, top, bottom };
  });
  await page.mouse.move(
    bounds.x + bounds.width * 0.6,
    bounds.y + bounds.height * 0.2,
  );
  const tooltip = page.getByRole("tooltip");
  await expect(tooltip).toContainText("Party A");
  const tooltipBounds = await tooltip.evaluate((element) => {
    const { top, bottom } = element.getBoundingClientRect();
    return { top, bottom };
  });
  expect(
    tooltipBounds.top >= bounds.bottom || tooltipBounds.bottom <= bounds.top,
  ).toBe(true);
});

test("party tooltips only open on hover and clicks do not focus pie slices", async ({
  backend,
  page,
  context,
}) => {
  await mockDetails(backend);
  await page.goto(`/funders/${funderId}`);
  const slice = page.locator(".funder-party-chart svg g").first();
  const legend = page.locator(".funder-party-legend li").first();
  const tooltip = page.getByRole("tooltip");
  await slice.hover();
  await expect(tooltip).toBeVisible();
  await slice.click();
  await expect(slice).not.toBeFocused();
  await page.getByRole("heading", { name: "Funding by party" }).hover();
  await expect(tooltip).toHaveCount(0);
  await slice.focus();
  await legend.focus();
  await expect(slice).not.toBeFocused();
  await expect(legend).not.toBeFocused();
  await expect(tooltip).toHaveCount(0);
  await page.keyboard.press("Enter");
  await expect(tooltip).toHaveCount(0);
  await page.keyboard.press("Space");
  await expect(tooltip).toHaveCount(0);
  await legend.hover();
  await expect(tooltip).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(tooltip).toHaveCount(0);
  await legend.click();
  await expect(tooltip).toHaveCount(0);
});

test("party tooltips do not open on touch", async ({
  backend,
  browser,
  baseURL,
}) => {
  const context = await browser.newContext({
    baseURL,
    hasTouch: true,
    viewport: { width: 800, height: 900 },
  });
  await mockDetails(backend);
  const page = await context.newPage();
  await page.goto(`/funders/${funderId}`);
  for (const target of [
    page.locator(".funder-party-legend li").first(),
    page.locator(".funder-party-chart svg g").first(),
  ]) {
    await target.tap();
    await expect(page.getByRole("tooltip")).toHaveCount(0);
    await expect(target).not.toBeFocused();
  }
  await context.close();
});

test("party tooltips close on scroll and resize", async ({
  backend,
  page,
  context,
}) => {
  await mockDetails(backend);
  await page.goto(`/funders/${funderId}`);
  const legend = page.locator(".funder-party-legend li").first();
  await legend.hover();
  await expect(page.getByRole("tooltip")).toBeVisible();
  await page.evaluate(() => window.scrollBy(0, 100));
  await expect(page.getByRole("tooltip")).toHaveCount(0);
  await legend.hover();
  await expect(page.getByRole("tooltip")).toBeVisible();
  await page.setViewportSize({ width: 1000, height: 800 });
  await expect(page.getByRole("tooltip")).toHaveCount(0);
});

test("member summary timeline merges same-month dates and retains negative monthly bars", async ({
  backend,
  page,
}) => {
  const declarations = [
    {
      ...memberHistoryFixture.declarations[0],
      category_name: "Employment",
      registered_at: "2026-01-02T00:00:00Z",
      entries: [
        { funder: null, amount: "10.25", currency: "GBP", payment_type: null },
      ],
    },
    {
      ...memberHistoryFixture.declarations[0],
      category_name: "Employment",
      registered_at: "2026-01-31T23:59:59Z",
      entries: [
        { funder: null, amount: "0.75", currency: "GBP", payment_type: null },
      ],
    },
    {
      ...memberHistoryFixture.declarations[0],
      category_name: "Employment",
      registered_at: "2026-03-31T00:00:00Z",
      entries: [
        { funder: null, amount: "-2.5", currency: "GBP", payment_type: null },
      ],
    },
  ];
  await backend.route("**/api/members/1", (route) =>
    route.fulfill({ json: { ...memberDetails, declarations } }),
  );
  await page.goto("/members/1");
  await expect(page.locator(".earnings-chart rect")).toHaveCount(3);
  await expect(page.locator(".earnings-chart rect title")).toHaveText([
    "Jan 2026: £11.00",
    "Feb 2026: £0.00",
    "Mar 2026: −£2.50",
  ]);
  await expect(page.locator(".earnings-chart rect.negative")).toHaveCount(1);
  expect(
    Number(
      await page
        .locator(".earnings-chart rect.negative")
        .getAttribute("height"),
    ),
  ).toBeGreaterThan(0);
  await expect(page.locator(".earnings-chart rect.negative")).toHaveAttribute(
    "y",
    (await page
      .locator(".earnings-chart .chart-baseline")
      .getAttribute("y1")) ?? "",
  );
});

test("funder summary keeps bounded party controls and the five authoritative recipients", async ({
  backend,
  page,
}) => {
  const recipients = Array.from({ length: 7 }, (_, index) => ({
    member: {
      ...member,
      id: String(index + 1),
      name: `Ranked recipient ${index + 1}`,
    },
    amount: String(700 - index * 100),
    entry_count: 1,
    unknown_amount_count: index === 0 ? 1 : 0,
  }));
  let partyCount = 1;
  await backend.route("**/api/funders/*", (route) =>
    route.fulfill({
      json: {
        ...funderDetails,
        currencies: [
          {
            currency: "GBP",
            amount: "1000",
            top_recipients: recipients,
            parties: Array.from({ length: partyCount }, (_, index) => ({
              party_id: index + 1,
              party_name: `Party ${index + 1} with a long name that needs the full accessible label`,
              amount: "100",
              entry_count: 1,
              unknown_amount_count: 0,
            })),
          },
        ],
      },
    }),
  );
  await page.goto(`/funders/${funderId}`);
  const chart = page.locator(".funder-party-chart");
  const legend = page.locator(".funder-party-legend");
  const initialChart = await chart.boundingBox();
  const initialLegend = await legend.boundingBox();
  expect(initialChart).not.toBeNull();
  expect(initialLegend?.height).toBe(144);
  await expect(page.locator(".funder-recipients tbody tr")).toHaveCount(5);
  await expect(page.locator(".funder-recipients tbody a")).toHaveText(
    recipients.slice(0, 5).map(({ member }) => member.name),
  );
  await expect(page.locator(".funder-recipients .partial-note")).toHaveText(
    "1 amount unavailable",
  );
  partyCount = 9;
  await page.reload();
  await expect(legend.locator("li")).toHaveCount(9);
  expect((await legend.boundingBox())?.height).toBe(initialLegend?.height);
  expect((await chart.boundingBox())?.height).toBe(initialChart?.height);
  await legend.locator("li").last().hover();
  await expect(page.getByRole("tooltip")).toContainText(
    "Party 9 with a long name that needs the full accessible label",
  );
  expect(await legend.evaluate((element) => element.scrollTop)).toBeGreaterThan(
    0,
  );
  await page.getByRole("tooltip").hover();
  await legend.evaluate((element) => {
    element.scrollTop = 0;
  });
  await expect(page.getByRole("tooltip")).toHaveCount(0);
  await legend.locator("li").first().hover();
  await legend.locator("li").last().hover();
  await expect(page.getByRole("tooltip")).toContainText(
    "Party 9 with a long name that needs the full accessible label",
  );
  await page.setViewportSize({ width: 375, height: 812 });
  await legend.locator("li").last().hover();
  await expect(page.getByRole("tooltip")).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(
    375,
  );
  partyCount = 0;
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.reload();
  await expect(chart).toContainText("No known funding totals are available.");
  expect((await chart.boundingBox())?.height).toBe(initialChart?.height);
  await expect(page.locator(".funder-recipients tbody tr")).toHaveCount(5);
});

test("funder chart combines Labour controls, counts and exact tooltip while retaining recipient parties", async ({
  backend,
  page,
}) => {
  const source = funderDetails.currencies[1];
  await backend.route("**/api/funders/*", (route) =>
    route.fulfill({
      json: {
        ...funderDetails,
        currencies: [
          {
            ...source,
            amount: "618900",
            parties: [
              {
                party_name: "Labour",
                amount: "546500",
                entry_count: 93,
                unknown_amount_count: 2,
              },
              {
                party_name: "Labour (Co-op)",
                amount: "72400",
                entry_count: 11,
                unknown_amount_count: 1,
              },
            ],
            top_recipients: [
              {
                ...source.top_recipients[0],
                member: { ...member, party_name: "Labour (Co-op)" },
              },
            ],
          },
        ],
      },
    }),
  );
  await page.goto(`/funders/${funderId}`);
  const controls = page.locator(".funder-party-legend li");
  await expect(controls).toHaveCount(1);
  await expect(controls).toContainText("Labour");
  await expect(controls).toContainText("104 entries, 3 amounts unavailable");
  await expect(controls).not.toContainText("Co-op");
  await controls.hover();
  await expect(page.getByRole("tooltip")).toContainText("£618,900.00");
  await expect(page.locator(".funder-party-chart svg g")).toHaveCount(1);
  await expect(page.locator(".funder-party-chart svg circle")).toHaveCount(1);
  await expect(page.locator(".funder-recipients")).toContainText(
    "Labour (Co-op)",
  );
});

test("initial HTML renders search and detail views while browser traffic stays inside Kit", async ({
  backend,
  browser,
  baseURL,
  page,
  request,
}) => {
  await mockDetails(backend);
  const html = await request.get("/search?q=Example&strictness=0.42");
  const rendered = await html.text();
  expect(rendered).toContain('class="entity-name"');
  expect(rendered).toContain("<title>Search MPs and funders · Exposed</title>");
  expect(rendered).toContain('name="description"');
  const context = await browser.newContext({
    baseURL,
    javaScriptEnabled: false,
  });
  const plainPage = await context.newPage();
  for (const path of [
    "/members/1",
    "/members/1/history",
    `/funders/${funderId}`,
    `/funders/${funderId}/history`,
  ]) {
    await plainPage.goto(path);
    await expect(
      plainPage.getByRole("heading", {
        name: path.startsWith("/members")
          ? member.name
          : funderDetails.funder.name,
        exact: true,
      }),
    ).toBeVisible();
    await expect(
      plainPage.getByRole("link", {
        name: path.endsWith("/history")
          ? "Summary"
          : path.startsWith("/members")
            ? "Declaration history"
            : "Funding history",
        exact: true,
      }),
    ).toBeVisible();
  }
  await context.close();
  const browserUrls: string[] = [];
  page.on("request", (incoming) => browserUrls.push(incoming.url()));
  await page.goto("/search?q=Example&strictness=0.42");
  await expect(page.getByRole("searchbox")).toBeEnabled();
  await page.getByRole("link", { name: member.name, exact: true }).click();
  await expect(
    page.getByRole("heading", { name: member.name, exact: true }),
  ).toBeVisible();
  expect(
    browserUrls.some(
      (url) =>
        new URL(url).pathname.startsWith("/api/") ||
        new URL(url).port === "6999",
    ),
  ).toBe(false);
  expect(browserUrls.every((url) => new URL(url).origin === baseURL)).toBe(
    true,
  );
});

test("member declared amounts include gifts and travel with registration coverage", async ({
  backend,
  page,
}) => {
  await mockDetails(backend);
  await backend.route("**/api/members/1", (route) =>
    route.fulfill({
      json: {
        ...memberDetails,
        summary: {
          ...memberDetails.summary,
          declared_totals: declaredTotals(
            { amount: "275", declaration_count: 2, incomplete: false },
            { amount: "425", declaration_count: 5, incomplete: false },
          ),
          first_registered_at: "2020-01-01T00:00:00Z",
        },
        declarations: [
          {
            ...memberDetails.declarations[0],
            category_name: "Employment",
            registered_at: "2024-07-09T00:00:00Z",
            entries: [
              {
                amount: "200",
                currency: "GBP",
                funder: null,
                payment_type: null,
              },
            ],
          },
          {
            ...memberDetails.declarations[0],
            category_name: "Gifts",
            registered_at: "2023-01-01T00:00:00Z",
            entries: [
              {
                amount: "100",
                currency: "GBP",
                funder: null,
                payment_type: null,
              },
            ],
          },
          {
            ...memberDetails.declarations[0],
            category_name: "Visits outside the UK",
            registered_at: null,
            entries: [
              {
                amount: "50",
                currency: "GBP",
                funder: null,
                payment_type: null,
              },
            ],
          },
          {
            ...memberDetails.declarations[0],
            category_name: "Visits outside the UK",
            registered_at: "2025-01-01T00:00:00Z",
            entries: [
              {
                amount: "75",
                currency: "GBP",
                funder: null,
                payment_type: null,
              },
            ],
          },
          {
            ...memberDetails.declarations[0],
            category_name: "Gifts",
            registered_at: "2020-01-01T00:00:00Z",
            entries: [],
          },
        ],
      },
    }),
  );
  await page.goto("/members/1");
  await expect(page.locator(".declared-value")).toHaveText([
    "£425.00",
    "£275.00",
    "£275.00",
  ]);
  await expect(
    page.getByRole("region", {
      name: "Declared this term in GBP",
      exact: true,
    }),
  ).toHaveText("£275.00");
  await expect(
    page.getByRole("region", {
      name: "Total declared (January 2020) in GBP",
      exact: true,
    }),
  ).toHaveText("£425.00");
  await expect(page.locator(".earnings-chart rect title")).toHaveText([
    "Jul 2024: £200.00",
  ]);
  await expect(
    page.locator(".member-declared-totals th[scope=row]"),
  ).toHaveText([
    "Total declared (January 2020)",
    "Declared this term",
    "Declared in 2026",
  ]);
  await expect(
    page.locator(".member-declared-totals .partial-note"),
  ).toHaveCount(0);
  if (process.env.EXPOSED_SCREENSHOT_DIR)
    await page.screenshot({
      path: `${process.env.EXPOSED_SCREENSHOT_DIR}/member-earnings-summary.png`,
      fullPage: true,
    });
  await page
    .getByRole("link", { name: "Declaration history", exact: true })
    .click();
  await expect(page.locator(".declared-value")).toHaveCount(0);
  if (process.env.EXPOSED_SCREENSHOT_DIR)
    await page.screenshot({
      path: `${process.env.EXPOSED_SCREENSHOT_DIR}/member-earnings-history.png`,
      fullPage: true,
    });
});

test("declared amounts keep precise figures readable on narrow screens and missing amounts neutral", async ({
  backend,
  page,
}) => {
  await mockDetails(backend);
  let amount: string | null = "1234567890.12";
  await backend.route("**/api/members/1", (route) =>
    route.fulfill({
      json: {
        ...memberDetails,
        summary: {
          ...memberDetails.summary,
          declared_totals: declaredTotals({
            amount,
            declaration_count: 1,
            incomplete: amount === null,
          }),
        },
        declarations: [
          {
            ...memberDetails.declarations[0],
            category_name: "Employment",
            entries: [
              { amount, currency: "GBP", funder: null, payment_type: null },
            ],
          },
        ],
      },
    }),
  );
  for (const width of [850, 801, 601, 390]) {
    await page.setViewportSize({ width, height: 900 });
    await page.goto("/members/1");
    await expect(page.locator(".declared-value")).toHaveText([
      "£1,234,567,890.12",
      "£1,234,567,890.12",
      "£1,234,567,890.12",
    ]);
    await page.evaluate(() => document.fonts.ready);
    const figures = await page
      .locator(".declared-value")
      .evaluateAll((elements) =>
        elements.map((element) => {
          const text = document.createRange();
          text.selectNodeContents(element);
          return {
            lines: text.getClientRects().length,
            fontSize: parseFloat(getComputedStyle(element).fontSize),
            fontWeight: getComputedStyle(element).fontWeight,
          };
        }),
      );
    for (const figure of figures) {
      expect(figure.lines).toBe(1);
      expect(figure.fontSize).toBeCloseTo(
        Math.min(44, Math.max(24, width * 0.05)),
      );
      expect(figure.fontWeight).toBe("650");
    }
  }
  const knownStyle = await page
    .locator(".declared-value")
    .first()
    .evaluate((element) => ({
      color: getComputedStyle(element).color,
      fontSize: parseFloat(getComputedStyle(element).fontSize),
    }));
  amount = null;
  await page.reload();
  await expect(page.locator(".declared-unavailable")).toHaveCount(3);
  const missingStyle = await page
    .locator(".declared-value")
    .first()
    .evaluate((element) => ({
      color: getComputedStyle(element).color,
      fontSize: parseFloat(getComputedStyle(element).fontSize),
      fontWeight: getComputedStyle(element).fontWeight,
    }));
  expect(missingStyle.color).not.toBe(knownStyle.color);
  expect(missingStyle.fontSize).toBe(20);
  expect(missingStyle.fontWeight).toBe("500");
  await expect(page.locator(".declared-value")).toHaveText([
    "Amount unavailable",
    "Amount unavailable",
    "Amount unavailable",
  ]);
  amount = "90071992547409939007199254740993.123456789";
  await page.reload();
  const figure = page.getByRole("region", {
    name: "Total declared (January 2026) in GBP",
    exact: true,
  });
  await expect(figure).toHaveText(
    "£90,071,992,547,409,939,007,199,254,740,993.123456789",
  );
  await figure.focus();
  await page.keyboard.press("ArrowRight");
  expect(
    await figure.evaluate((element) => {
      const range = document.createRange();
      range.selectNodeContents(element);
      return range.getClientRects().length;
    }),
  ).toBe(1);
  expect(
    await figure.evaluate((element) =>
      parseFloat(getComputedStyle(element).outlineWidth),
    ),
  ).toBeGreaterThanOrEqual(2);
  await expect
    .poll(() => figure.evaluate((element) => element.scrollLeft))
    .toBeGreaterThan(0);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
});

test("declared amounts distinguish an empty term, historical zero and unknown amounts", async ({
  backend,
  page,
}) => {
  await mockDetails(backend);
  let registeredAt = "2023-01-01T00:00:00Z";
  let amount: string | null = "0";
  let incomplete = false;
  await backend.route("**/api/members/1", (route) =>
    route.fulfill({
      json: {
        ...memberDetails,
        summary: {
          ...memberDetails.summary,
          declared_totals: declaredTotals(
            registeredAt === "2023-01-01T00:00:00Z"
              ? { amount: null, declaration_count: 0, incomplete: false }
              : { amount, declaration_count: 1, incomplete },
            { amount, declaration_count: 1, incomplete },
          ),
        },
        declarations: [
          {
            ...memberDetails.declarations[0],
            category_name: "Employment",
            registered_at: registeredAt,
            entries: [
              { amount, currency: "GBP", funder: null, payment_type: null },
              ...(incomplete
                ? [
                    {
                      amount: null,
                      currency: "GBP",
                      funder: null,
                      payment_type: null,
                    },
                  ]
                : []),
            ],
          },
        ],
      },
    }),
  );
  await page.goto("/members/1");
  await expect(page.locator(".declared-value")).toHaveText([
    "£0.00",
    "No declaration data available",
    "No declaration data available",
  ]);
  registeredAt = "2025-01-01T00:00:00Z";
  await page.reload();
  await expect(page.locator(".declared-value")).toHaveText([
    "£0.00",
    "£0.00",
    "£0.00",
  ]);
  incomplete = true;
  await page.reload();
  await expect(page.locator(".declared-value")).toHaveText([
    "£0.00",
    "£0.00",
    "£0.00",
  ]);
  await expect(page.locator(".declared-value").first()).toHaveAttribute(
    "title",
    "Known GBP amounts only. Some amounts unavailable.",
  );
  amount = null;
  await page.reload();
  await expect(page.locator(".declared-value")).toHaveText([
    "Amount unavailable",
    "Amount unavailable",
    "Amount unavailable",
  ]);
});

test("malformed details show an error and recover after retry", async ({
  backend,
  page,
}) => {
  let calls = 0;
  await backend.route("**/api/members/1", (route) =>
    route.fulfill({
      json:
        ++calls === 1
          ? { ...memberDetails, member: { ...member, id: "2" } }
          : memberDetails,
    }),
  );
  await page.goto("/members/1");
  await expect(page.getByRole("alert")).toContainText("unexpected response");
  await page.getByRole("button", { name: "Try again" }).click();
  await expect(
    page.getByRole("heading", { name: member.name, exact: true }),
  ).toBeVisible();
});

test("declared table uses backend year and UTC historical month before member navigation", async ({
  backend,
  page,
}) => {
  await backend.route("**/api/members/1", (route) =>
    route.fulfill({
      json: {
        ...memberDetails,
        summary: {
          ...memberDetails.summary,
          first_registered_at: "2020-01-31T23:30:00-02:00",
          declared_totals: {
            ...memberDetails.summary.declared_totals,
            calendar_year: 2040,
            current_year: {
              amount: "12.3456789",
              declaration_count: 2,
              incomplete: true,
            },
          },
        },
      },
    }),
  );
  await page.goto("/members/1");
  const table = page.getByRole("table", { name: "Declared GBP amounts" });
  await expect(table.getByRole("rowheader")).toHaveText([
    "Total declared (February 2020)",
    "Declared this term",
    "Declared in 2040",
  ]);
  await expect(table.getByRole("row").last()).toContainText("£12.3456789");
  await expect(
    table.getByText("Known GBP amounts only. Some amounts unavailable."),
  ).toHaveCount(0);
  await expect(
    table.getByRole("region", { name: "Declared in 2040 in GBP", exact: true }),
  ).toHaveAttribute(
    "title",
    "Known GBP amounts only. Some amounts unavailable.",
  );
  const tableBounds = await table.boundingBox();
  const metadata = await page.locator(".member-identity-info").boundingBox();
  const tabs = await page
    .getByRole("navigation", { name: "Member views" })
    .boundingBox();
  expect(
    metadata &&
      tableBounds &&
      tabs &&
      metadata.y + metadata.height <= tableBounds.y &&
      tableBounds.y + tableBounds.height <= tabs.y,
  ).toBeTruthy();
  if (process.env.EXPOSED_SCREENSHOT_DIR) {
    await page.screenshot({
      path: `${process.env.EXPOSED_SCREENSHOT_DIR}/mp-declared-table-desktop.png`,
      fullPage: true,
    });
    await page.setViewportSize({ width: 390, height: 844 });
    await page.screenshot({
      path: `${process.env.EXPOSED_SCREENSHOT_DIR}/mp-declared-table-narrow.png`,
      fullPage: true,
    });
  }
});
