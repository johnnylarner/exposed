import { registrationTimeline } from "../src/lib/registration-timeline";
import { expect, test } from "@playwright/test";
import { summarizeMember, formatPounds } from "../src/lib/member-summary";
import {
  categoryBars,
  type MemberDetails,
  type MemberDeclaration,
  type FundingEntry,
} from "../src/lib/entity-details";

function entry(
  amount: string | null,
  currency: string | null = "GBP",
): FundingEntry {
  return { amount, currency, funder: null, paymentType: null };
}
function declaration(
  registeredAt: string | null,
  entries: FundingEntry[],
  categoryName = "Employment and earnings",
  categoryId = 12,
): MemberDeclaration {
  return { registeredAt, entries, categoryName, categoryId, sourceId: 1 };
}
function details(declarations: MemberDeclaration[]): MemberDetails {
  return {
    member: {
      id: "1",
      name: "Example",
      partyName: "",
      membershipFrom: "",
      isCurrentCommons: true,
    },
    declarations,
    summary: {
      declaredTotals: {
        calendarYear: 2031,
        currentYear: {
          amount: "12.34",
          declarationCount: 2,
          incomplete: false,
        },
        currentTermStart: "2024-07-09T00:00:00Z",
        currentTerm: {
          amount: "225.45",
          declarationCount: 7,
          incomplete: true,
        },
        historical: { amount: "333", declarationCount: 9, incomplete: true },
      },
      currencies: [
        {
          currency: "EUR",
          amount: "999",
          categories: [],
          entryCount: 1,
          unknownAmountCount: 0,
        },
        {
          currency: "GBP",
          amount: "123.45",
          categories: [],
          entryCount: 1,
          unknownAmountCount: 0,
        },
      ],
      entryCount: 1,
      unknownCurrencyCount: 0,
      firstRegisteredAt: null,
      lastRegisteredAt: null,
      undatedDeclarationCount: 0,
    },
  };
}

test("sources preserve exact amounts across categories, dates and currency coverage", () => {
  const funder = { id: "a", name: "Same source" };
  const sourced = (amount: string | null, currency: string | null = "GBP") => ({
    ...entry(amount, currency),
    funder,
  });
  const fixture = details([
    declaration("2026-01-05Z", [
      sourced("9007199254740993.123456789"),
      sourced("9007199254740993.123456789"),
    ]),
    declaration(
      null,
      [
        sourced("0.000000001"),
        sourced(null),
        sourced("99", null),
        sourced("100", "EUR"),
      ],
      "Unknown category",
      999,
    ),
    declaration("1900-01-01Z", [
      sourced("-0.000000001"),
      entry("0"),
      entry(null),
    ]),
  ]);
  const summary = summarizeMember(fixture);
  expect(summary.total).toBe("123.45");
  expect(summary.declaredTotals).toBe(fixture.summary.declaredTotals);
  expect(summary.categories).toBe(fixture.summary.currencies[1].categories);
  expect(summary.sources).toEqual([
    {
      funder,
      amount: "18014398509481986.246913578",
      entryCount: 7,
      missingGbpAmountCount: 1,
      unknownCurrencyCount: 1,
      foreignCurrencyCount: 1,
    },
    {
      funder: null,
      amount: "0",
      entryCount: 2,
      missingGbpAmountCount: 1,
      unknownCurrencyCount: 0,
      foreignCurrencyCount: 0,
    },
  ]);
  expect(formatPounds(summary.sources[0].amount)).toBe(
    "£18,014,398,509,481,986.246913578",
  );
  expect(formatPounds("-5")).toBe("−£5.00");
  expect(formatPounds(null)).toBe("Amount unavailable");
});

test("sources rank signed exact values, then names and IDs, with null last", () => {
  const values = [
    ["d", "Same", "0", "GBP"],
    ["c", "Same", "-0.00", "GBP"],
    ["b", "Beta", "9007199254740993.000000002", "GBP"],
    ["a", "Alpha", "9007199254740993.000000001", "GBP"],
    ["e", "Negative", "-1", "GBP"],
    ["f", "Foreign", "999999999999999999", "EUR"],
    ["g", "Missing", null, "GBP"],
    ["h", "Unknown", "100", null],
    ["i", "Whitespace", "200", "  "],
  ] satisfies [string, string, string | null, string | null][];
  const summary = summarizeMember(
    details([
      declaration(
        null,
        values.map(([id, name, amount, currency]) => ({
          ...entry(amount, currency),
          funder: { id, name },
        })),
      ),
    ]),
  );
  expect(summary.sources.map(({ funder }) => funder?.id)).toEqual([
    "b",
    "a",
    "c",
    "d",
    "e",
    "f",
    "g",
    "h",
    "i",
  ]);
  expect(summary.sources.map(({ amount }) => amount)).toEqual([
    "9007199254740993.000000002",
    "9007199254740993.000000001",
    "0.00",
    "0",
    "-1",
    null,
    null,
    null,
    null,
  ]);
  expect(summarizeMember(details([])).sources).toEqual([]);
});

test("month iteration includes leap February after a January 31 registration", () => {
  expect(
    registrationTimeline([
      declaration("2024-01-31T23:59:59Z", [entry("1")]),
      declaration("2024-02-29T00:00:00Z", [entry("2")]),
      declaration("2024-03-31T23:59:59Z", [entry("3")]),
    ]),
  ).toEqual({
    months: [
      { start: "2024-01-01", amount: "1", incomplete: false },
      { start: "2024-02-01", amount: "2", incomplete: false },
      { start: "2024-03-01", amount: "3", incomplete: false },
    ],
    undated: false,
  });
});

test("category shares preserve decimal magnitudes without cancellation", () => {
  const amounts = ["0.08", "-0.0200", "0", "-0.00", null];
  expect(
    categoryBars(
      amounts.map((amount) => ({ amount })),
      "total",
    ),
  ).toEqual([
    { width: 80, negative: false },
    { width: 20, negative: true },
    { width: 0, negative: false },
    { width: 0, negative: false },
    { width: 0, negative: false },
  ]);
  expect(categoryBars([{ amount: "5" }, { amount: "-5" }], "total")).toEqual([
    { width: 50, negative: false },
    { width: 50, negative: true },
  ]);
  const thirds = categoryBars(
    Array.from({ length: 3 }, () => ({ amount: "1" })),
    "total",
  );
  for (const bar of thirds) expect(bar.width).toBeCloseTo(100 / 3, 7);
  expect(thirds.reduce((sum, bar) => sum + bar.width, 0)).toBeCloseTo(100, 7);
});

test("empty and unavailable amounts have no bar for either basis", () => {
  for (const basis of ["total", "maximum"] satisfies ("total" | "maximum")[]) {
    expect(categoryBars([], basis)).toEqual([]);
    expect(
      categoryBars(
        [{ amount: null }, { amount: "0" }, { amount: "-0.00" }],
        basis,
      ),
    ).toEqual([
      { width: 0, negative: false },
      { width: 0, negative: false },
      { width: 0, negative: false },
    ]);
  }
});

test("category ratios remain finite beyond Number precision and range", () => {
  const huge = "9".repeat(400);
  expect(categoryBars([{ amount: huge }, { amount: huge }], "total")).toEqual([
    { width: 50, negative: false },
    { width: 50, negative: false },
  ]);
  expect(
    categoryBars(
      [
        { amount: "9007199254740993.123456789" },
        { amount: "27021597764222979.370370367" },
      ],
      "total",
    ),
  ).toEqual([
    { width: 25, negative: false },
    { width: 75, negative: false },
  ]);
});

test("weekly bars retain maximum scaling and hundredth-percent truncation", () => {
  expect(
    categoryBars(
      [
        { amount: "3" },
        { amount: "-1" },
        { amount: "0.0001" },
        { amount: null },
      ],
      "maximum",
    ),
  ).toEqual([
    { width: 100, negative: false },
    { width: 33.33, negative: true },
    { width: 0, negative: false },
    { width: 0, negative: false },
  ]);
  const weeks = Array.from({ length: 200 }, () => ({ amount: "1" }));
  expect(categoryBars(weeks, "maximum").every((bar) => bar.width === 100)).toBe(
    true,
  );
});

test("funding timeline includes every category with exact UTC months and missing coverage", () => {
  const timeline = registrationTimeline([
    declaration(
      "2026-01-04T23:59:59Z",
      [entry("9007199254740993.123456789")],
      "Gifts",
    ),
    declaration(
      "2026-01-05T00:00:00Z",
      [entry("-2.5"), entry("0.5"), entry(null), entry("900", null)],
      "Donations",
    ),
    declaration("2026-03-19T00:00:00Z", [entry(null)], "Visits"),
    declaration(null, [entry("5")], "Gifts"),
    declaration("2027-01-01T00:00:00Z", [entry("99", "EUR")], "Gifts"),
  ]);
  expect(timeline).toEqual({
    undated: true,
    months: [
      {
        start: "2026-01-01",
        amount: "9007199254740991.123456789",
        incomplete: true,
      },
      { start: "2026-02-01", amount: "0", incomplete: false },
      { start: "2026-03-01", amount: null, incomplete: true },
    ],
  });
  expect(
    registrationTimeline([declaration(null, [entry("99", "EUR")])]),
  ).toEqual({ months: [], undated: false });
});

test("declared headlines pass backend authority through despite contradictory declarations", () => {
  for (const declarations of [
    [],
    [declaration(null, [entry("99999999999999999999")], "Gifts")],
  ]) {
    const fixture = details(declarations);
    const summary = summarizeMember(fixture);
    expect(summary.declaredTotals).toBe(fixture.summary.declaredTotals);
    expect(summary.declaredTotals.calendarYear).toBe(2031);
    expect(summary.declaredTotals.currentYear.amount).toBe("12.34");
    expect(summary.declaredTotals.currentTerm.amount).toBe("225.45");
    expect(summary.declaredTotals.historical.amount).toBe("333");
  }
});

test("headline null, zero, counts and coverage pass through without inference", () => {
  const fixture = details([declaration("2025-01-01Z", [entry("100")])]);
  fixture.summary.declaredTotals = {
    calendarYear: 2031,
    currentYear: { amount: null, declarationCount: 1, incomplete: true },
    currentTermStart: "2024-07-09T00:00:00Z",
    currentTerm: { amount: null, declarationCount: 0, incomplete: false },
    historical: { amount: "0", declarationCount: 8, incomplete: true },
  };
  expect(summarizeMember(fixture).declaredTotals).toBe(
    fixture.summary.declaredTotals,
  );
});

test("summary preserves backend period metadata without a local boundary", () => {
  const fixture = details([declaration("2025-01-01Z", [entry("99")])]);
  fixture.summary.declaredTotals.currentTermStart = "2040-01-01T00:00:00Z";
  expect(summarizeMember(fixture).declaredTotals).toBe(
    fixture.summary.declaredTotals,
  );
});
