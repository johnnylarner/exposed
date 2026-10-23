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

test("summary preserves duplicate arbitrary precision values and server pounds total", () => {
  const summary = summarizeMember(
    details([
      declaration("2026-01-05T00:00:00Z", [
        entry("9007199254740993.123456789"),
        entry("9007199254740993.123456789"),
        entry("0.000000001"),
      ]),
      declaration("2026-01-31T23:59:59Z", [entry("-0.000000001")]),
    ]),
  );
  expect(summary.total).toBe("123.45");
  expect(summary.earnings).toEqual([
    {
      start: "2026-01-01",
      amount: "18014398509481986.246913578",
      incomplete: false,
    },
  ]);
  expect(formatPounds(summary.earnings[0].amount)).toBe(
    "£18,014,398,509,481,986.246913578",
  );
  expect(formatPounds("-5")).toBe("−£5.00");
  expect(formatPounds(null)).toBe("Amount unavailable");
});

test("UTC registration months preserve negatives, zero, gaps and incomplete values", () => {
  const summary = summarizeMember(
    details([
      declaration("2025-12-31T23:00:00-02:00", [entry("-2.5")]),
      declaration("2025-12-01T00:30:00+02:00", [entry("0"), entry(null)]),
      declaration("2025-12-20T00:00:00Z", [entry("-5")]),
      declaration("2026-01-19T00:00:00Z", [entry("4"), entry("900", null)]),
      declaration("2026-03-26T00:00:00Z", [entry(null)]),
      declaration(null, [entry("88")]),
      declaration("2027-01-01T00:00:00Z", [entry("77", "EUR")]),
    ]),
  );
  expect(summary.earnings).toEqual([
    { start: "2025-11-01", amount: "0", incomplete: true },
    { start: "2025-12-01", amount: "-5", incomplete: false },
    { start: "2026-01-01", amount: "1.5", incomplete: true },
    { start: "2026-02-01", amount: "0", incomplete: false },
    { start: "2026-03-01", amount: null, incomplete: true },
  ]);
  expect(summary.undated).toBe(true);
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

test("earnings labels own classification across legacy IDs and current label variants", () => {
  const summary = summarizeMember(
    details([
      declaration("2026-01-05Z", [entry("1")], "Employment and earnings", 12),
      declaration(
        "2026-01-05Z",
        [entry("2")],
        "Employment and earnings - Ad hoc payments",
        99,
      ),
      declaration(
        "2026-01-05Z",
        [entry("3")],
        "Employment and earnings - Ongoing paid employment",
        2,
      ),
      declaration("2026-01-05Z", [entry("4")], "Employment", 1),
      declaration("2026-01-05Z", [entry("100")], "Gifts", 1),
    ]),
  );
  expect(summary.earnings[0].amount).toBe("10");
  expect(summarizeMember(details([])).earnings).toEqual([]);
  const undated = summarizeMember(details([declaration(null, [entry("3")])]));
  expect(undated.earnings).toEqual([]);
  expect(undated.undated).toBe(true);
  expect(
    summarizeMember(details([declaration("2026-01-05Z", [entry("0")])]))
      .earnings[0].amount,
  ).toBe("0");
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

test("backend headlines stay independent of the earnings timeline", () => {
  const fixture = details([
    declaration("2025-01-01T00:00:00Z", [entry("10.25")]),
    declaration("2025-01-02T00:00:00Z", [entry("20.50")], "Gifts"),
  ]);
  const summary = summarizeMember(fixture);
  expect(summary.declaredTotals).toBe(fixture.summary.declaredTotals);
  expect(summary.earnings).toEqual([
    { start: "2025-01-01", amount: "10.25", incomplete: false },
  ]);
});

test("summary preserves backend period metadata without a local boundary", () => {
  const fixture = details([declaration("2025-01-01Z", [entry("99")])]);
  fixture.summary.declaredTotals.currentTermStart = "2040-01-01T00:00:00Z";
  expect(summarizeMember(fixture).declaredTotals).toBe(
    fixture.summary.declaredTotals,
  );
});
