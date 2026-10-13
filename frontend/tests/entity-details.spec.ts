import { expect, test, type BrowserContext } from "@playwright/test";

const funderId = "10000000-0000-0000-0000-000000000001";
const member = {
  id: "1",
  name: "Ava Example",
  party_id: 1,
  party_name: "Party A",
  membership_from: "Example constituency",
  is_current_commons: true,
};
const memberHistoryFixture = {
  member,

  declarations: [
    {
      source_id: 120,
      category_id: 3,
      category_name: "Support",
      registered_at: "2026-01-21T00:00:00Z",
      entries: [
        {
          funder: { id: funderId, name: "Exact Funder" },
          amount: "9007199254740993.123456789",
          currency: "GBP",
          payment_type: "Cash",
        },
        {
          funder: { id: funderId, name: "Exact Funder" },
          amount: "9007199254740993.123456789",
          currency: "GBP",
          payment_type: "Cash",
        },
        { funder: null, amount: null, currency: null, payment_type: null },
      ],
    },
    {
      source_id: 119,
      category_id: 987,
      category_name: "Visits",
      registered_at: null,
      entries: [],
    },
  ],
};
const memberDetails = {
  ...memberHistoryFixture,
  summary: {
    entry_count: 3,
    unknown_currency_count: 1,
    first_registered_at: "2026-01-21T00:00:00Z",
    last_registered_at: "2026-01-21T00:00:00Z",
    undated_declaration_count: 1,
    currencies: [
      {
        currency: "GBP",
        amount: "18014398509481986.246913578",
        entry_count: 2,
        unknown_amount_count: 0,
        categories: [
          {
            category_id: 3,
            category_name: "Support",
            amount: "18014398509481986.246913578",
            entry_count: 2,
            unknown_amount_count: 0,
          },
          {
            category_id: 987,
            category_name: "Unrecognized source category",
            amount: "-5",
            entry_count: 1,
            unknown_amount_count: 0,
          },
        ],
      },
      {
        currency: "EUR",
        amount: null,
        entry_count: 1,
        unknown_amount_count: 1,
        categories: [
          {
            category_id: 4,
            category_name: "Gifts",
            amount: null,
            entry_count: 1,
            unknown_amount_count: 1,
          },
        ],
      },
      {
        currency: "JPY",
        amount: "0",
        entry_count: 1,
        unknown_amount_count: 0,
        categories: [
          {
            category_id: 4,
            category_name: "Gifts",
            amount: "0",
            entry_count: 1,
            unknown_amount_count: 0,
          },
        ],
      },
    ],
  },
};
const funderDetails = {
  funder: {
    id: funderId,
    name: "Exact Funder",
    funder_kind: "Company",
    company_number: "00123456",
    aliases: [" Exact Funder ", "EXACT FUNDER", "Exact Funder"],
  },
  declarations: [
    {
      member,
      declaration: {
        ...memberHistoryFixture.declarations[0],
        entries: memberHistoryFixture.declarations[0].entries.slice(0, 2),
      },
    },
  ],
  entry_count: 7,
  unknown_currency_count: 2,
  recipient_limit: 10,
  currencies: [
    {
      currency: "EUR",
      amount: null,
      parties: [
        {
          party_id: 2,
          party_name: "Party B",
          amount: null,
          entry_count: 1,
          unknown_amount_count: 1,
        },
      ],
      top_recipients: [
        {
          member: {
            ...member,
            id: "2",
            name: "Bea Example",
            party_name: "Party B",
          },
          amount: null,
          entry_count: 1,
          unknown_amount_count: 1,
        },
      ],
    },
    {
      currency: "GBP",
      amount: "18014398509481981.246913578",
      parties: [
        {
          party_id: 1,
          party_name: "Party A",
          amount: "18014398509481986.246913578",
          entry_count: 3,
          unknown_amount_count: 1,
        },
        {
          party_id: 2,
          party_name: "Party B",
          amount: "-5",
          entry_count: 1,
          unknown_amount_count: 0,
        },
      ],
      top_recipients: [
        {
          member,
          amount: "18014398509481986.246913578",
          entry_count: 3,
          unknown_amount_count: 1,
        },
        {
          member: {
            ...member,
            id: "2",
            name: "Bea Example",
            party_name: "Party B",
          },
          amount: "0",
          entry_count: 1,
          unknown_amount_count: 0,
        },
      ],
    },
    {
      currency: "USD",
      amount: "500",
      parties: [
        {
          party_id: 2,
          party_name: "Party B",
          amount: "500",
          entry_count: 1,
          unknown_amount_count: 0,
        },
      ],
      top_recipients: [
        {
          member: {
            ...member,
            id: "2",
            name: "Bea Example",
            party_name: "Party B",
          },
          amount: "500",
          entry_count: 1,
          unknown_amount_count: 0,
        },
      ],
    },
  ],
};

async function mockDetails(context: BrowserContext) {
  await context.route("**/api/search?*", (route) =>
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
  await context.route("**/api/members/*", (route) =>
    route.fulfill({ json: memberDetails }),
  );
  await context.route("**/api/funders/*", (route) =>
    route.fulfill({ json: funderDetails }),
  );
}

test("native links preserve search and threshold through details, back, reload and new tabs", async ({
  page,
  context,
}) => {
  await mockDetails(context);
  await page.goto("/?q=Example&strictness=0.42");
  const link = page.getByRole("link", { name: "Ava Example", exact: true });
  await expect(link).toHaveAttribute(
    "href",
    "/members/1?q=Example&strictness=0.42",
  );
  await expect(page.locator(".entity-name button")).toHaveCount(0);
  const popupPromise = context.waitForEvent("page");
  await link.click({ button: "middle" });
  const popup = await popupPromise;
  await expect(
    popup.getByRole("heading", { name: member.name, exact: true }),
  ).toBeVisible();
  await popup.close();
  await link.click();
  await expect(page).toHaveURL(/\/members\/1\?q=Example&strictness=0.42$/u);
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
    new RegExp(`/funders/${funderId}\\?q=Example&strictness=0.42$`, "u"),
  );
  await page.getByRole("link", { name: "Ava Example", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: member.name, exact: true }),
  ).toBeVisible();
  await page.getByRole("link", { name: "Back to search", exact: true }).click();
  await expect(page.getByRole("searchbox")).toHaveValue("Example");
  await expect(page.getByRole("slider")).toHaveValue("0.42");
  await link.click();
  await expect(
    page.getByRole("heading", { name: member.name, exact: true }),
  ).toBeVisible();
  await page.goBack();
  await expect(page.getByRole("searchbox")).toHaveValue("Example");
});

test("member summary shows recent declarations when none are donations", async ({
  page,
}) => {
  await page.route("**/api/members/1", (route) =>
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
  page,
}) => {
  const declarations = Array.from({ length: 7 }, (_, index) => ({
    ...memberHistoryFixture.declarations[0],
    source_id: 120 - index,
    category_id: index === 0 ? 3 : 1,
    category_name: index === 0 ? "Support" : "Employment",
    entries: [],
  }));
  await page.route("**/api/members/1", (route) =>
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
  page,
  context,
}) => {
  await mockDetails(context);
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
  page,
  context,
}) => {
  await mockDetails(context);
  await page.goto(`/funders/${funderId}?q=Example&strictness=0.42`);
  await expect(page.locator(".member-total")).toHaveText(
    "£18,014,398,509,481,981.246913578",
  );
  await expect(page.getByText("Company number 00123456")).toBeVisible();
  const controls = page.locator(".funder-party-legend button");
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
  await controls.nth(1).focus();
  await expect(page.getByRole("tooltip")).toContainText("−£5.00");
  await page.keyboard.press("Escape");
  await controls.nth(1).click();
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
  ).toHaveAttribute(
    "href",
    `/funders/${funderId}/history?q=Example&strictness=0.42`,
  );
});

test("funding history retains sources, duplicate amounts and date paging", async ({
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
  await page.route("**/api/funders/*", (route) =>
    route.fulfill({ json: { ...funderDetails, declarations } }),
  );
  await page.goto(`/funders/${funderId}/history?q=Example&strictness=0.42`);
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
  ).toHaveAttribute("href", "/members/1?q=Example&strictness=0.42");
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
    new RegExp(`/funders/${funderId}\\?q=Example&strictness=0.42$`, "u"),
  );
});

test("funder boundary rejects missing totals and malformed source history", async ({
  page,
}) => {
  for (const body of [
    { ...funderDetails, declarations: undefined },
    {
      ...funderDetails,
      declarations: [
        {
          member,
          declaration: {
            ...memberHistoryFixture.declarations[0],
            registered_at: "broken",
          },
        },
      ],
    },
    {
      ...funderDetails,
      currencies: [{ ...funderDetails.currencies[1], amount: undefined }],
    },
  ]) {
    await page.route("**/api/funders/*", (route) =>
      route.fulfill({ json: body }),
    );
    await page.goto(`/funders/${funderId}`);
    await expect(page.getByRole("alert")).toContainText("unexpected response");
    await page.unroute("**/api/funders/*");
  }
});

test("former MPs retain their latest membership description without a constituency label", async ({
  page,
}) => {
  await page.route("**/api/members/1", (route) =>
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
  page,
}) => {
  let calls = 0;
  await page.route("**/api/members/*", async (route) => {
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
  await page.route("**/api/members/999", (route) =>
    route.fulfill({ status: 404 }),
  );
  await page.goto("/members/999?q=Example");
  await expect(
    page.getByRole("heading", { name: "Entity not found" }),
  ).toBeVisible();
  await expect(
    page.getByRole("link", { name: "Search MPs and funders", exact: true }),
  ).toHaveAttribute("href", "/?q=Example");
  await page.goto("/members/0");
  await expect(
    page.getByRole("heading", { name: "Entity not found" }),
  ).toBeVisible();
  expect(calls).toBe(2);
});

test("empty details and unknown currency only details remain distinct from errors", async ({
  page,
}) => {
  await page.route("**/api/members/1", (route) =>
    route.fulfill({
      json: {
        ...memberDetails,
        declarations: [],
        summary: {
          ...memberDetails.summary,
          currencies: [],
          entry_count: 0,
          unknown_currency_count: 0,
        },
      },
    }),
  );
  await page.route("**/api/funders/*", (route) =>
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
  await page.goto(`/funders/${funderId}`);
  await expect(
    page.getByText("No funding entries are stored for this funder."),
  ).toBeVisible();
  await page.route("**/api/funders/*", (route) =>
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

test("malformed responses and mismatched identities fail at the boundary", async ({
  page,
}) => {
  await page.route("**/api/members/*", (route) =>
    route.fulfill({
      json: { ...memberDetails, member: { ...member, id: "2" } },
    }),
  );
  await page.goto("/members/1");
  await expect(page.getByRole("alert")).toContainText("unexpected response");
  await page.route("**/api/funders/*", (route) =>
    route.fulfill({
      json: {
        ...funderDetails,
        currencies: [
          {
            currency: "GBP",
            parties: [
              {
                party_name: "Party A",
                amount: 9007199254740993,
                entry_count: 1,
                unknown_amount_count: 0,
              },
            ],
            top_recipients: [],
          },
        ],
      },
    }),
  );
  await page.goto(`/funders/${funderId}`);
  await expect(page.getByRole("alert")).toContainText("unexpected response");
});

test("long names and exact amounts fit mobile and source markup stays literal", async ({
  page,
}) => {
  const longName =
    'Association <img src=x onerror="alert(1)"> for Research and Public Interest Organisations';
  await page.setViewportSize({ width: 360, height: 780 });
  await page.route("**/api/funders/*", (route) =>
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
  await page.route("**/api/members/*", (route) =>
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
  page,
}) => {
  await page.clock.install();
  let release: () => void = () => {};
  const pending = new Promise<void>((resolve) => {
    release = resolve;
  });
  await page.route("**/api/members/*", async (route) => {
    await pending;
    await route.fulfill({ json: memberDetails }).catch(() => {});
  });
  await page.goto("/members/1?q=Example");
  await expect(
    page.getByRole("heading", { name: "Loading details" }),
  ).toBeVisible();
  await page.clock.fastForward(10_001);
  await expect(page.getByRole("alert")).toContainText(
    "Details are taking too long",
  );
  await page.getByRole("button", { name: "Try again" }).click();
  await expect(
    page.getByRole("heading", { name: "Loading details" }),
  ).toBeVisible();
  await page.route("**/api/search?*", (route) =>
    route.fulfill({ json: { entities: [] } }),
  );
  await page.getByRole("link", { name: "Back to search", exact: true }).click();
  release();
  await page.clock.fastForward(300);
  await expect(page.getByRole("searchbox")).toHaveValue("Example");
  await expect(
    page.getByRole("heading", { name: member.name, exact: true }),
  ).toHaveCount(0);
});

test("recorded names retain stored variants for every funder kind and hide empty lists", async ({
  page,
}) => {
  for (const kind of ["Company", "Individual", null]) {
    await page.route("**/api/funders/*", (route) =>
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
  await page.route("**/api/funders/*", (route) =>
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

for (const aliases of [null, "Exact Funder", ["Exact Funder", 42]]) {
  test(`malformed recorded names ${JSON.stringify(aliases)} fail at the boundary`, async ({
    page,
  }) => {
    await page.route("**/api/funders/*", (route) =>
      route.fulfill({
        json: {
          ...funderDetails,
          funder: { ...funderDetails.funder, aliases },
        },
      }),
    );
    await page.goto(`/funders/${funderId}`);
    await expect(page.getByRole("alert")).toContainText("unexpected response");
    await expect(
      page.getByRole("region", { name: "Recorded names" }),
    ).toHaveCount(0);
  });
}

for (const negative of [false, true]) {
  test(`category bars show full-track shares with negative amounts ${negative}`, async ({
    page,
  }) => {
    await page.route("**/api/members/1", (route) =>
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

test("member summary displays server totals, signed category bars, zero and unavailable amounts", async ({
  page,
  context,
}) => {
  await mockDetails(context);
  await page.goto("/members/1");
  await expect(page.locator(".member-total")).toHaveText(
    "£18,014,398,509,481,986.246913578",
  );
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
    "Known declared value",
  );
  const total = await page.locator(".member-total").boundingBox();
  const tabs = await page.locator(".member-tabs").boundingBox();
  expect(total && tabs && total.y + total.height <= tabs.y).toBeTruthy();
  await expect(page.locator(".declarations-list > li")).toHaveCount(2);
  await expect(
    page.getByRole("link", { name: "View declaration history" }),
  ).toHaveAttribute("href", "/members/1/history");
});

test("history filters the full dataset before paging and resets page for date changes", async ({
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
  await page.route("**/api/members/1", (route) =>
    route.fulfill({ json: { ...memberDetails, declarations } }),
  );
  await page.goto("/members/1/history?q=Example&strictness=0.42");
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
  await expect(page).toHaveURL(/members\/1\?q=Example&strictness=0.42$/u);
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
  page,
}) => {
  await page.route("**/api/funders/*", (route) =>
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
  await page.locator(".funder-party-legend button").first().focus();
  await expect(page.getByRole("tooltip")).toContainText("£0.00");
  await page.locator(".funder-party-legend button").nth(1).focus();
  await expect(page.getByRole("tooltip")).toContainText("Amount unavailable");
  await expect(page.locator(".funder-party-chart path")).toHaveCount(0);
});

for (const amounts of [
  ["3", "7"],
  ["0", "-5"],
  [null, "0"],
]) {
  test(`party controls expose totals for ${amounts.join("/")}`, async ({
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
    await page.route("**/api/funders/*", (route) =>
      route.fulfill({ json: { ...funderDetails, currencies } }),
    );
    await page.goto(`/funders/${funderId}`);
    await expect(page.locator(".funder-party-legend")).not.toContainText("£");
    for (const [index, amount] of amounts.entries()) {
      await page.locator(".funder-party-legend button").nth(index).click();
      await expect(page.getByRole("tooltip")).toContainText(
        amount === null
          ? "Amount unavailable"
          : amount === "-5"
            ? "−£5.00"
            : `£${amount}.00`,
      );
      await page.keyboard.press("Escape");
    }
    if (amounts[0] === "3") {
      await expect(page.locator(".funder-party-chart svg path")).toHaveCount(2);
      await page.locator(".funder-party-chart svg g").nth(1).focus();
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

test("party tooltip toggles on touch and closes outside or on scroll", async ({
  browser,
}) => {
  const context = await browser.newContext({
    hasTouch: true,
    viewport: { width: 800, height: 900 },
  });
  await mockDetails(context);
  const page = await context.newPage();
  await page.goto(`/funders/${funderId}`);
  const button = page.locator(".funder-party-legend button").first();
  await button.tap();
  await expect(page.getByRole("tooltip")).toBeVisible();
  await button.tap();
  await expect(page.getByRole("tooltip")).toHaveCount(0);
  await button.tap();
  await page.getByRole("heading", { name: "Funding by party" }).tap();
  await expect(page.getByRole("tooltip")).toHaveCount(0);
  await button.tap();
  await page.evaluate(() => window.scrollBy(0, 100));
  await expect(page.getByRole("tooltip")).toHaveCount(0);
  await context.close();
});

test("member summary timeline merges same-month dates and retains negative monthly bars", async ({
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
  await page.route("**/api/members/1", (route) =>
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
  await page.route("**/api/funders/*", (route) =>
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
  await expect(legend.locator("button")).toHaveCount(9);
  expect((await legend.boundingBox())?.height).toBe(initialLegend?.height);
  expect((await chart.boundingBox())?.height).toBe(initialChart?.height);
  await legend.locator("button").last().focus();
  await expect(page.getByRole("tooltip")).toContainText(
    "Party 9 with a long name that needs the full accessible label",
  );
  expect(await legend.evaluate((element) => element.scrollTop)).toBeGreaterThan(
    0,
  );
  await legend.evaluate((element) => {
    element.scrollTop = 0;
  });
  await expect(page.getByRole("tooltip")).toHaveCount(0);
  await legend.locator("button").first().focus();
  await legend.locator("button").last().focus();
  await expect(page.getByRole("tooltip")).toContainText(
    "Party 9 with a long name that needs the full accessible label",
  );
  await page.setViewportSize({ width: 375, height: 812 });
  await legend.locator("button").last().click();
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
  page,
}) => {
  const source = funderDetails.currencies[1];
  await page.route("**/api/funders/*", (route) =>
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
  const controls = page.locator(".funder-party-legend button");
  await expect(controls).toHaveCount(1);
  await expect(controls).toContainText("Labour");
  await expect(controls).toContainText("104 entries, 3 amounts unavailable");
  await expect(controls).not.toContainText("Co-op");
  await controls.focus();
  await expect(page.getByRole("tooltip")).toContainText("£618,900.00");
  await expect(page.locator(".funder-party-chart svg g")).toHaveCount(1);
  await expect(page.locator(".funder-party-chart svg circle")).toHaveCount(1);
  await expect(page.locator(".funder-recipients")).toContainText(
    "Labour (Co-op)",
  );
});
