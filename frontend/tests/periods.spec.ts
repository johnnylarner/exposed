import { test, expect } from "./backend";
import {
  memberDetails,
  funderDetails,
  funderId,
  declaredTotals,
} from "./details-fixtures";
import { rankingFixture } from "./rankings-fixtures";
import {
  reportingPeriods,
  isReportingPeriod,
  type ReportingPeriod,
} from "../src/lib/reporting-period";

const scopes = {
  historical: { amount: "9007199254740993.123456789", count: 21 },
  "current-term": { amount: "0", count: 12 },
  "current-year": { amount: "-12.34", count: 1 },
} satisfies Record<ReportingPeriod, { amount: string; count: number }>;
const totals = declaredTotals(
  {
    amount: scopes["current-term"].amount,
    declaration_count: 12,
    incomplete: false,
  },
  { amount: scopes.historical.amount, declaration_count: 21, incomplete: true },
  {
    amount: scopes["current-year"].amount,
    declaration_count: 1,
    incomplete: false,
  },
);

for (const kind of ["member", "funder"] as const) {
  test(`${kind} selects one authoritative headline and scoped history, resets dates and paging`, async ({
    backend,
    page,
  }) => {
    const path = kind === "member" ? "/members/1" : `/funders/${funderId}`;
    const reads: string[] = [];
    await backend.route(`**/api${path}?*`, (route) => {
      const period = new URL(route.request().url()).searchParams.get("period");
      if (!isReportingPeriod(period))
        throw new Error("Missing typed SSR period");
      reads.push(period);
      const declarations = Array.from(
        { length: scopes[period].count },
        (_, index) => ({
          ...memberDetails.declarations[0],
          source_id: 500 - index,
          category_name: `Selected ${period}`,
          registered_at: "2026-01-21T00:00:00Z",
        }),
      );
      return route.fulfill({
        json:
          kind === "member"
            ? {
                ...memberDetails,
                period,
                declarations,
                summary: {
                  ...memberDetails.summary,
                  declared_totals: totals,
                  entry_count: scopes[period].count,
                },
              }
            : {
                ...funderDetails,
                period,
                declared_totals: totals,
                declarations: declarations.map((declaration) => ({
                  member: memberDetails.member,
                  declaration,
                })),
                entry_count: scopes[period].count,
              },
      });
    });
    const relatedPath =
      kind === "member" ? `/funders/${funderId}` : "/members/1";
    await backend.route(`**/api${relatedPath}?*`, (route) => {
      const period = new URL(route.request().url()).searchParams.get("period");
      return route.fulfill({
        json:
          kind === "member"
            ? { ...funderDetails, period }
            : { ...memberDetails, period },
      });
    });
    await page.goto(`${path}?q=Example`);
    const selector = page.getByRole("navigation", { name: "Reporting period" });
    await expect(
      selector.getByRole("link", {
        name: "This parliamentary term",
        exact: true,
      }),
    ).toHaveAttribute("aria-current", "page");
    await expect(page.locator(".declared-totals .member-total")).toHaveText([
      "£0.00",
    ]);
    expect(reads.at(-1)).toBe("current-term");
    await page
      .getByRole("link", {
        name: kind === "member" ? "Declaration history" : "Funding history",
        exact: true,
      })
      .click();
    await expect(page).toHaveURL(
      `${path}/history?period=current-term&q=Example`,
    );
    await page.getByRole("button", { name: "Next", exact: true }).click();
    await expect(
      page.getByText("Showing 11 to 12 of 12 declarations"),
    ).toBeVisible();
    await page.getByLabel("Registered from").fill("2026-01-01");
    await page.getByLabel("Registered from").dispatchEvent("change");
    await expect(
      page.getByText("Showing 1 to 10 of 12 declarations"),
    ).toBeVisible();
    await selector
      .getByRole("link", { name: "Historical", exact: true })
      .click();
    await expect(page.getByLabel("Registered from")).toHaveValue("");
    await expect(
      page.getByText("Showing 1 to 10 of 21 declarations"),
    ).toBeVisible();
    await expect(page.locator(".declared-totals .member-total")).toHaveText([
      "£9,007,199,254,740,993.123456789",
    ]);
    await expect(
      page.locator(".declared-totals [role=region]"),
    ).toHaveAttribute(
      "title",
      "Known GBP amounts only. Some amounts unavailable.",
    );
    await selector
      .getByRole("link", { name: "This year", exact: true })
      .click();
    await expect(
      page.getByText("Showing 1 to 1 of 1 declarations"),
    ).toBeVisible();
    await expect(page.locator(".declared-totals .member-total")).toHaveText([
      "−£12.34",
    ]);
    await page.reload();
    expect(reads.at(-1)).toBe("current-year");
    const historyRelated = page
      .locator(".declaration")
      .first()
      .getByRole("link", {
        name: kind === "member" ? "Exact Funder" : "Ava Example",
        exact: true,
      })
      .first();
    await expect(historyRelated).toHaveAttribute(
      "href",
      `${relatedPath}?period=current-year&q=Example`,
    );
    await historyRelated.click();
    await expect(page).toHaveURL(
      `${relatedPath}?period=current-year&q=Example`,
    );
    await page.goBack();
    await expect(page).toHaveURL(
      `${path}/history?period=current-year&q=Example`,
    );
    await page.getByRole("link", { name: "Summary", exact: true }).click();
    await expect(page).toHaveURL(`${path}?period=current-year&q=Example`);
    const related = page
      .getByRole("link", {
        name: kind === "member" ? "Exact Funder" : "Ava Example",
        exact: true,
      })
      .first();
    await expect(related).toHaveAttribute(
      "href",
      `${kind === "member" ? `/funders/${funderId}` : "/members/1"}?period=current-year&q=Example`,
    );
    for (const width of [1440, 320]) {
      await page.setViewportSize({ width, height: 1000 });
      expect(
        await page.evaluate(() => document.documentElement.scrollWidth),
      ).toBeLessThanOrEqual(width);
      const metadata = await page
        .locator(".member-identity-info")
        .boundingBox();
      const controls = await selector.boundingBox();
      const metric = await page.locator(".declared-totals").boundingBox();
      expect(controls!.y).toBeGreaterThan(metadata!.y + metadata!.height);
      expect(metric!.y).toBeGreaterThan(controls!.y + controls!.height);
    }
    const historical = selector.getByRole("link", {
      name: "Historical",
      exact: true,
    });
    await historical.focus();
    await expect(historical).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(
      selector.getByRole("link", { name: "Historical", exact: true }),
    ).toHaveAttribute("aria-current", "page");
  });
}

test("rankings preserve period and independent category across reload, history and entity links", async ({
  backend,
  page,
}) => {
  const reads: string[] = [];
  await backend.route("**/api/rankings?*", (route) => {
    const period = new URL(route.request().url()).searchParams.get("period");
    if (!isReportingPeriod(period)) throw new Error("Missing period");
    reads.push(period);
    return route.fulfill({ json: { ...rankingFixture, period } });
  });
  await page.goto("/rankings?category=companies&period=historical");
  await expect(
    page.getByRole("list", { name: "Companies rankings" }),
  ).toBeVisible();
  await expect(
    page.getByRole("link", { name: "Example company" }),
  ).toHaveAttribute(
    "href",
    "/funders/10000000-0000-0000-0000-000000000002?period=historical",
  );
  await page.getByRole("link", { name: "This year", exact: true }).click();
  await expect(page).toHaveURL(/category=companies&period=current-year/u);
  await page.getByRole("button", { name: "MPs", exact: true }).click();
  await expect(page).toHaveURL(/category=members&period=current-year/u);
  await page.reload();
  await expect(page.getByRole("list", { name: "MPs rankings" })).toBeVisible();
  expect(reads.at(-1)).toBe("current-year");
  await page.goBack();
  await expect(
    page.getByRole("list", { name: "Companies rankings" }),
  ).toBeVisible();
});

test("invalid explicit periods reject before API reads on every route", async ({
  request,
  backend,
}) => {
  let reads = 0;
  await backend.route("**/api/*", (route) => {
    reads += 1;
    return route.fulfill({ status: 500 });
  });
  for (const path of [
    "/rankings",
    "/members/1",
    `/funders/${funderId}/history`,
  ]) {
    for (const period of ["", "other", "Historical"]) {
      const response = await request.get(`${path}?period=${period}`);
      expect(response.status()).toBe(400);
    }
  }
  expect(reads).toBe(0);
});

test("a period mismatch or failure cannot display previous scope numbers", async ({
  page,
  backend,
}) => {
  await backend.route("**/api/members/1?*", (route) =>
    route.fulfill({ json: memberDetails }),
  );
  await page.goto("/members/1");
  await expect(page.locator(".declared-totals")).toBeVisible();
  await page
    .getByRole("link", { name: reportingPeriods["current-year"], exact: true })
    .click();
  await expect(page.getByRole("alert")).toContainText("unexpected response");
  await expect(page.locator(".declared-totals")).toHaveCount(0);
});

for (const state of [
  {
    name: "empty",
    amount: null,
    count: 0,
    incomplete: false,
    expected: "No declarations recorded",
  },
  {
    name: "unknown",
    amount: null,
    count: 1,
    incomplete: true,
    expected: "Amount unavailable",
  },
  { name: "zero", amount: "0", count: 1, incomplete: true, expected: "£0.00" },
] as const) {
  for (const kind of ["member", "funder"] as const) {
    test(`${kind} selected headline preserves ${state.name} independently of populated other periods`, async ({
      page,
      backend,
    }) => {
      const path = kind === "member" ? "/members/1" : `/funders/${funderId}`;
      const total = declaredTotals(
        { amount: "1", declaration_count: 1, incomplete: false },
        { amount: "200", declaration_count: 2, incomplete: false },
        {
          amount: state.amount,
          declaration_count: state.count,
          incomplete: state.incomplete,
        },
      );
      await backend.route(`**/api${path}?*`, (route) =>
        route.fulfill({
          json:
            kind === "member"
              ? {
                  ...memberDetails,
                  period: "current-year",
                  summary: { ...memberDetails.summary, declared_totals: total },
                }
              : {
                  ...funderDetails,
                  period: "current-year",
                  declared_totals: total,
                },
        }),
      );
      await page.goto(`${path}?period=current-year`);
      await expect(page.locator(".declared-totals .member-total")).toHaveText([
        state.expected,
      ]);
      if (state.amount !== null)
        await expect(
          page.locator(".declared-totals [role=region]"),
        ).toHaveAttribute(
          "title",
          "Known GBP amounts only. Some amounts unavailable.",
        );
      else
        await expect(
          page.locator(".declared-totals [role=region]"),
        ).toHaveCount(0);
    });
  }
}
