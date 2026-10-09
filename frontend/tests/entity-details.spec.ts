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
const memberDetails = {
  member,
  declaration_count: 24,
  declaration_limit: 20,
  declarations: [
    {
      source_id: 120,
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
      category_name: "Visits",
      registered_at: null,
      entries: [],
    },
  ],
};
const funderDetails = {
  funder: {
    id: funderId,
    name: "Exact Funder",
    funder_kind: "Company",
    company_number: "00123456",
    aliases: [" Exact Funder ", "EXACT FUNDER", "Exact Funder"],
  },
  entry_count: 7,
  unknown_currency_count: 2,
  recipient_limit: 10,
  currencies: [
    {
      currency: "EUR",
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
    page.getByRole("heading", { name: "Declared support by party" }).first(),
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

test("member declarations retain duplicate precise amounts and missing source fields", async ({
  page,
  context,
}) => {
  await mockDetails(context);
  await page.goto("/members/1");
  await expect(page.locator(".profile-type")).toHaveText("MP");
  await expect(
    page.getByText("Showing 2 of 24 stored declarations, most recent first."),
  ).toBeVisible();
  await expect(page.locator(".funding-value .amount")).toHaveText([
    "9,007,199,254,740,993.123456789",
    "9,007,199,254,740,993.123456789",
    "Amount unavailable",
  ]);
  for (const text of [
    "21 Jan 2026",
    "Registration date unavailable",
    "Funder unavailable",
    "Currency unavailable",
    "Payment type unavailable",
    "No funding entries recorded.",
  ])
    await expect(page.getByText(text, { exact: true })).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Top recipients" }),
  ).toHaveCount(0);
});

test("funder currencies keep exact totals, local missing amounts, zero and negatives", async ({
  page,
  context,
}) => {
  await mockDetails(context);
  await page.goto(`/funders/${funderId}`);
  await expect(page.locator(".profile-type")).toHaveText("Funder");
  await expect(page.getByText("00123456", { exact: true })).toBeVisible();
  await expect(
    page.getByText(
      "2 entries have an unknown currency and are excluded from totals and rankings.",
    ),
  ).toBeVisible();
  const gbp = page.getByRole("region", {
    name: "GBP declared support",
    exact: true,
  });
  await expect(gbp.locator(".amount")).toHaveText([
    "18,014,398,509,481,986.246913578",
    "−5",
    "18,014,398,509,481,986.246913578",
    "0",
  ]);
  await expect(gbp.locator(".partial-note")).toHaveText([
    "1 amount unavailable",
    "1 amount unavailable",
  ]);
  const eur = page.getByRole("region", {
    name: "EUR declared support",
    exact: true,
  });
  await expect(eur.locator(".amount")).toHaveText([
    "Amount unavailable",
    "Amount unavailable",
  ]);
  await expect(eur.locator(".partial-note")).toHaveText([
    "1 amount unavailable",
    "1 amount unavailable",
  ]);
  await expect(
    page
      .getByRole("region", { name: "USD declared support" })
      .getByRole("link", { name: "Bea Example" }),
  ).toBeVisible();
  await expect(page.locator(".summary-context")).toContainText(
    "Parties do not describe membership",
  );
  await expect(
    page.getByRole("heading", { name: "Recent declarations" }),
  ).toHaveCount(0);
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
  await expect(
    page.getByText("Latest membership", { exact: true }),
  ).toBeVisible();
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
      json: { ...memberDetails, declarations: [], declaration_count: 0 },
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
    page.getByText("No known currencies are available for totals or rankings."),
  ).toBeVisible();
  await expect(
    page.getByText(
      "2 entries have an unknown currency and are excluded from totals and rankings.",
    ),
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
