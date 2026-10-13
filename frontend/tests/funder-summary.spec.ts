import { expect, test } from "@playwright/test";
import { partyChart } from "../src/lib/funder-summary";
import type { FunderDetails } from "../src/lib/entity-details";

type Party = FunderDetails["currencies"][number]["parties"][number];
function party(partyName: string, amount: string | null): Party {
  return {
    partyName,
    amount,
    entryCount: 1,
    unknownAmountCount: amount === null ? 1 : 0,
  };
}

test("party colours stay fixed across order changes and Labour grouping", () => {
  const expected = [
    ["Labour", "#e4003b"],
    ["Conservative", "#0087dc"],
    ["Reform UK", "#8b5cf6"],
    ["Liberal Democrat", "#ffe500"],
    ["Green Party", "#00a85a"],
    ["Scottish National Party", "#c7a600"],
  ];
  const input = expected.map(([name]) => party(name, "10"));
  input.push(party("Labour (Co-op)", "5"));
  for (const parties of [input, [...input].reverse()]) {
    const result = partyChart(parties);
    for (const [name, color] of expected) {
      expect(result.find(({ party }) => party.partyName === name)?.color).toBe(
        color,
      );
    }
  }
  expect(
    partyChart([party("Other", "10"), party("constructor", "10")]).map(
      ({ color }) => color,
    ),
  ).toEqual(["#df8550", "#6587ad"]);
});

test("Labour grouping preserves first order, counts, unrelated records and source data", () => {
  const input = [
    party("Other", "5.00"),
    {
      ...party("Labour (Co-op)", "72400"),
      entryCount: 11,
      unknownAmountCount: 2,
    },
    party("Other", "10"),
    { ...party("Labour", "546500"), entryCount: 93, unknownAmountCount: 3 },
    party("Labour Co-op", "1"),
  ];
  const original = structuredClone(input);
  input.forEach(Object.freeze);
  Object.freeze(input);
  expect(partyChart(input).map(({ party }) => party)).toEqual([
    original[0],
    {
      partyName: "Labour",
      amount: "618900",
      entryCount: 104,
      unknownAmountCount: 5,
    },
    original[2],
    original[4],
  ]);
  expect(input).toEqual(original);
  expect(
    partyChart([party("Labour (Co-op)", "12.50")])[0].party.partyName,
  ).toBe("Labour");
  expect(partyChart([])).toEqual([]);
});

test("Labour decimals add exactly beyond Number precision and range", () => {
  const huge = "9".repeat(400);
  for (const [left, right, expected] of [
    ["9007199254740993.123456789", "0.876543211", "9007199254740994"],
    ["0.01", "0.00200", "0.012"],
    [huge, "1.00001", `1${"0".repeat(400)}.00001`],
  ]) {
    const result = partyChart([
      party("Labour", left),
      party("Labour (Co-op)", right),
    ]);
    expect(result[0].party.amount).toBe(expected);
    expect(result[0].share).toBe(1);
  }
});

test("Labour unknown and signed totals remain distinct before pie weighting", () => {
  for (const [left, right, amount, share, unknownAmountCount] of [
    [null, null, null, 0, 2],
    [null, "12.50", "12.5", 0.135135, 1],
    ["12.50", null, "12.5", 0.135135, 1],
    ["100", "-20", "80", 0.5, 0],
    ["20", "-100", "-80", 0, 0],
    ["0.01", "-0.0100", "0", 0, 0],
    ["-0.00", "0", "0", 0, 0],
    ["-0.02", "0.01", "-0.01", 0, 0],
  ] satisfies [string | null, string | null, string | null, number, number][]) {
    const result = partyChart([
      party("Labour", left),
      party("Labour (Co-op)", right),
      party("Other", "80"),
    ]);
    expect(result[0].party).toEqual({
      partyName: "Labour",
      amount,
      entryCount: 2,
      unknownAmountCount,
    });
    expect(result[0].share).toBe(share);
    expect(result[1].share).toBeCloseTo(share === 0 ? 1 : 1 - share, 5);
  }
});
