import { expect, test } from "@playwright/test";
import { overviewFixture } from "./overview-fixture";
import { parseOverview, loadOverview } from "../src/lib/server/overview";
import { overviewCharts } from "../src/lib/overview";

test("preserves exact amounts beyond Number precision and party identity", () => {
  const fixture = overviewFixture();
  fixture.top_members[0].total.amount = "9007199254740993.123456789";
  fixture.parties[1].party_name = fixture.parties[0].party_name;
  const result = parseOverview(fixture);
  const charts = overviewCharts(result);
  expect(result.topMembers[0].member.membershipFrom).toBe(
    "Richmond and Northallerton",
  );
  expect(result.topMembers[0].total.amount).toBe("9007199254740993.123456789");
  expect(charts.members[0].value).toBe("£9,007,199,254,740,993.123456789");
  expect(charts.parties).toHaveLength(3);
  expect(charts.parties[2].value).not.toBe("£0.00");
  expect(charts.members.every((bar) => Number.isFinite(bar.width))).toBe(true);
});
test("negative party values make pie unavailable without losing values", () => {
  const fixture = overviewFixture();
  fixture.parties[0].total.amount = "-1.01";
  const charts = overviewCharts(parseOverview(fixture));
  expect(charts.negativeParty).toBe(true);
  expect(charts.parties[0].value).toBe("−£1.01");
  expect(charts.parties.every((slice) => slice.share === 0)).toBe(true);
});
test("zero and all-negative quartiles produce finite geometry", () => {
  for (const amount of ["0", "-123.45"]) {
    const fixture = overviewFixture();
    for (const quartile of fixture.quartiles)
      quartile.minimum = quartile.maximum = amount;
    for (const member of fixture.top_members) member.total.amount = amount;
    const charts = overviewCharts(parseOverview(fixture));
    expect(
      charts.quartiles.every(
        (bar) => Number.isFinite(bar.y) && Number.isFinite(bar.height),
      ),
    ).toBe(true);
    expect(charts.members.every((bar) => Number.isFinite(bar.width))).toBe(
      true,
    );
  }
});
test("rejects malformed coverage and nonempty null ranges", () => {
  const fixture = overviewFixture();
  fixture.member_count++;
  expect(() => parseOverview(fixture)).toThrow();
  const nullRange = {
    ...overviewFixture(),
    quartiles: overviewFixture().quartiles.map((q) => ({
      ...q,
      minimum: null,
    })),
  };
  expect(() => parseOverview(nullRange)).toThrow();
});
test("backend failure leaves a usable unavailable state", async () => {
  const fetcher: typeof fetch = async () => new Response("", { status: 503 });
  expect(
    await loadOverview(
      new AbortController().signal,
      fetcher,
      "http://example.test",
    ),
  ).toEqual({ kind: "unavailable" });
});

test("an empty cohort retains four unavailable quartiles", () => {
  const fixture = overviewFixture();
  const empty = parseOverview({
    ...fixture,
    member_count: 0,
    unavailable_member_count: 0,
    incomplete: false,
    quartiles: fixture.quartiles.map((q) => ({
      ...q,
      member_count: 0,
      minimum: null,
      maximum: null,
      incomplete_member_count: 0,
    })),
    top_members: [],
    parties: [],
  });
  const charts = overviewCharts(empty);
  expect(charts.members).toEqual([]);
  expect(charts.parties).toEqual([]);
  expect(charts.quartiles.every((bar) => bar.height === 0)).toBe(true);
});
