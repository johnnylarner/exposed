import { expect, test } from "@playwright/test";
import { searchEntities } from "../src/lib/server/search";
import { loadDetails } from "../src/lib/server/entity-details";
import { parseRankings, loadRankings } from "../src/lib/server/rankings";
import { rankingRows } from "../src/lib/rankings";
import { completeRankingFixture, rankingFixture } from "./rankings-fixtures";
import {
  funderId,
  funderDetails,
  member,
  memberDetails,
} from "./details-fixtures";

function reply(body: unknown): typeof fetch {
  return async () => Response.json(body);
}
const signal = () => new AbortController().signal;
const funder = {
  id: funderId,
  name: "Exact Funder",
  kind: "Funder",
  funder_kind: "Company",
  match_source: { kind: "name" },
};

for (const match_source of [
  undefined,
  null,
  {},
  { kind: "other" },
  { kind: "alias", name: "" },
  { kind: "alias", name: "   " },
  { kind: "alias" },
]) {
  test(`search rejects malformed match source ${JSON.stringify(match_source)}`, async () => {
    await expect(
      searchEntities(
        "Exact",
        signal(),
        reply({ entities: [funder] }),
        "http://api",
      ),
    ).resolves.toHaveLength(1);
    await expect(
      searchEntities(
        "Exact",
        signal(),
        reply({ entities: [{ ...funder, match_source }] }),
        "http://api",
      ),
    ).rejects.toThrow("unexpected match source");
  });
}

test("search rejects alias provenance for an MP", async () => {
  const mp = {
    id: "1",
    name: "West",
    kind: "MP",
    funder_kind: null,
    match_source: { kind: "name" },
  };
  await expect(
    searchEntities("West", signal(), reply({ entities: [mp] }), "http://api"),
  ).resolves.toHaveLength(1);
  await expect(
    searchEntities(
      "West",
      signal(),
      reply({
        entities: [
          { ...mp, match_source: { kind: "alias", name: "West Midlands" } },
        ],
      }),
      "http://api",
    ),
  ).rejects.toThrow("unexpected MP identity");
});

const funderRoute = {
  kind: "funder",
  id: funderId,
  view: "summary",
} satisfies Parameters<typeof loadDetails>[0];
for (const [name, body] of [
  ["missing declarations", { ...funderDetails, declarations: undefined }],
  ["missing declared totals", { ...funderDetails, declared_totals: undefined }],
  [
    "missing first registration",
    { ...funderDetails, first_registered_at: undefined },
  ],
  [
    "invalid source timestamp",
    {
      ...funderDetails,
      declarations: [
        {
          member,
          declaration: {
            ...funderDetails.declarations[0].declaration,
            registered_at: "broken",
          },
        },
      ],
    },
  ],
  [
    "missing currency total",
    {
      ...funderDetails,
      currencies: funderDetails.currencies.map((group, index) =>
        index === 1 ? { ...group, amount: undefined } : group,
      ),
    },
  ],
  ...[null, "Exact Funder", ["Exact Funder", 42]].map((aliases) => [
    `recorded names ${JSON.stringify(aliases)}`,
    { ...funderDetails, funder: { ...funderDetails.funder, aliases } },
  ]),
]) {
  test(`details rejects ${name}`, async () => {
    expect(
      (
        await loadDetails(
          funderRoute,
          signal(),
          reply(funderDetails),
          "http://api",
        )
      ).kind,
    ).toBe("funder");
    expect(
      await loadDetails(funderRoute, signal(), reply(body), "http://api"),
    ).toEqual({
      kind: "error",
      message: "Details returned an unexpected response. Try again.",
    });
  });
}

test("details rejects a mismatched member identity", async () => {
  const route = {
    kind: "member",
    id: "1",
    view: "summary",
  } satisfies Parameters<typeof loadDetails>[0];
  expect(
    (await loadDetails(route, signal(), reply(memberDetails), "http://api"))
      .kind,
  ).toBe("member");
  expect(
    (
      await loadDetails(
        route,
        signal(),
        reply({ ...memberDetails, member: { ...member, id: "2" } }),
        "http://api",
      )
    ).kind,
  ).toBe("error");
});

test("details preserves decimal strings and rejects a numeric nested party amount", async () => {
  const valid = await loadDetails(
    funderRoute,
    signal(),
    reply(funderDetails),
    "http://api",
  );
  expect(valid.kind).toBe("funder");
  if (valid.kind !== "funder") throw new Error("expected funder details");
  expect(valid.details.currencies[1].parties[0].amount).toBe("24691.34");
  const body = {
    ...funderDetails,
    currencies: funderDetails.currencies.map((group, index) =>
      index === 1
        ? {
            ...group,
            parties: group.parties.map((party, partyIndex) =>
              partyIndex === 0 ? { ...party, amount: 12345.67 } : party,
            ),
          }
        : group,
    ),
  };
  expect(
    (await loadDetails(funderRoute, signal(), reply(body), "http://api")).kind,
  ).toBe("error");
});

for (const [name, fields] of [
  ["missing calendar year", { calendar_year: undefined }],
  ["string calendar year", { calendar_year: "2026" }],
  ["fractional calendar year", { calendar_year: 2026.5 }],
  ["null calendar year", { calendar_year: null }],
  ["missing current year", { current_year: undefined }],
  ["null current year", { current_year: null }],
  [
    "numeric year amount",
    { current_year: { amount: 12, declaration_count: 1, incomplete: false } },
  ],
  [
    "missing year amount",
    { current_year: { declaration_count: 1, incomplete: false } },
  ],
  [
    "negative year count",
    { current_year: { amount: "0", declaration_count: -1, incomplete: false } },
  ],
  [
    "missing year coverage",
    { current_year: { amount: null, declaration_count: 1 } },
  ],
] satisfies [string, Record<string, unknown>][]) {
  test(`details rejects ${name}`, async () => {
    const route = {
      kind: "member",
      id: "1",
      view: "summary",
    } satisfies Parameters<typeof loadDetails>[0];
    const valid = await loadDetails(
      route,
      signal(),
      reply(memberDetails),
      "http://api",
    );
    expect(valid.kind).toBe("member");
    if (valid.kind !== "member") throw new Error("expected member details");
    expect(valid.details.summary.declaredTotals.calendarYear).toBe(2026);
    expect(valid.details.summary.declaredTotals.currentYear.amount).toBe(
      "24691.34",
    );
    const body = {
      ...memberDetails,
      summary: {
        ...memberDetails.summary,
        declared_totals: {
          ...memberDetails.summary.declared_totals,
          ...fields,
        },
      },
    };
    expect(
      await loadDetails(
        funderRoute,
        signal(),
        reply({
          ...funderDetails,
          declared_totals: { ...funderDetails.declared_totals, ...fields },
        }),
        "http://api",
      ),
    ).toEqual({
      kind: "error",
      message: "Details returned an unexpected response. Try again.",
    });
    expect(
      await loadDetails(route, signal(), reply(body), "http://api"),
    ).toEqual({
      kind: "error",
      message: "Details returned an unexpected response. Try again.",
    });
  });
}

test("rankings parse all four boards without changing exact amounts or ordering", async () => {
  const parsed = await loadRankings(
    reply(rankingFixture),
    signal(),
    "http://api",
  );
  expect(parsed.members.map((row) => row.id)).toEqual(
    rankingFixture.members.map((row) => row.id),
  );
  expect(parsed.members[0].total.amount).toBe("9007199254740993.123456789");
  expect(rankingRows(parsed, "members")[0].amount).toBe(
    "£9,007,199,254,740,993.123456789",
  );
  expect(rankingRows(parsed, "members")[0].width).toBe(100);
  expect(rankingRows(parsed, "companies")[0].negative).toBe(true);
  expect(rankingRows(parsed, "parties")[0].href).toBeUndefined();
  expect(parsed.parties[0].total.amount).toBeNull();
  expect(parsed.individuals[0].total.amount).toBe("0");
});
for (const value of [
  { ...rankingFixture, companies: undefined },
  {
    ...rankingFixture,
    members: [...rankingFixture.members, rankingFixture.members[0]],
  },
  { ...rankingFixture, current_term_start: "yesterday" },
  { ...rankingFixture, members: [{ ...rankingFixture.members[0], id: "0" }] },
  {
    ...rankingFixture,
    individuals: [{ ...rankingFixture.individuals[0], id: "x" }],
  },
  ...[0, "1e9", undefined, "NaN"].map((amount) => ({
    ...rankingFixture,
    parties: [
      {
        ...rankingFixture.parties[0],
        total: { amount, declaration_count: 1, incomplete: false },
      },
    ],
  })),
  ...[-1, 1.5, undefined].map((declaration_count) => ({
    ...rankingFixture,
    companies: [
      {
        ...rankingFixture.companies[0],
        total: { amount: "1", declaration_count, incomplete: false },
      },
    ],
  })),
  {
    ...rankingFixture,
    parties: [
      {
        ...rankingFixture.parties[0],
        total: { amount: "1", declaration_count: 1 },
      },
    ],
  },
]) {
  test(`rankings reject malformed complete snapshots ${JSON.stringify(value)}`, () => {
    expect(() => parseRankings(value)).toThrow("unexpected response");
  });
}

import { parseUpdates, loadUpdates } from "../src/lib/server/updates";
import { updatesFixture } from "./updates-fixtures";

test("updates parse the full list and preserve counts and dates", async () => {
  const parsed = parseUpdates(updatesFixture);
  expect(parsed.members).toHaveLength(12);
  expect(parsed.members[0]).toEqual({
    id: "1",
    name: "Member 1",
    declarationCount: 2,
    latestRegistration: "2026-10-31T23:59:59Z",
  });
  expect(
    await loadUpdates(reply(updatesFixture), signal(), "http://backend"),
  ).toEqual(parsed);
  await expect(
    loadUpdates(
      async () => new Response(null, { status: 503 }),
      signal(),
      "http://backend",
    ),
  ).rejects.toThrow("Latest declarations couldn’t load");
});
for (const patch of [
  { month_start: "2026-10-02T00:00:00Z" },
  { month_start: "2026-10-01T00:00:00.000000001Z" },
  { month_end: "2026-12-01T00:00:00Z" },
  { month_end: "2026-11-01T00:00:00.000000001Z" },
  { members: [{ ...updatesFixture.members[0], declaration_count: 0 }] },
  { members: [{ ...updatesFixture.members[0], declaration_count: 1.5 }] },
  { members: [{ ...updatesFixture.members[0], id: "2147483648" }] },
  { members: [updatesFixture.members[0], updatesFixture.members[0]] },
  {
    members: [
      {
        ...updatesFixture.members[0],
        latest_registration: "2026-11-01T00:00:00Z",
      },
    ],
  },
  {
    members: [
      {
        ...updatesFixture.members[0],
        latest_registration: "2026-09-30T23:59:59Z",
      },
    ],
  },
  {
    members: [
      {
        ...updatesFixture.members[0],
        latest_registration: "2026-10-32T00:00:00Z",
      },
    ],
  },
  { companies: [{ ...updatesFixture.companies[0], id: "bad" }] },
]) {
  test(`updates reject malformed snapshot ${JSON.stringify(patch)}`, () => {
    expect(() => parseUpdates({ ...updatesFixture, ...patch })).toThrow(
      "unexpected response",
    );
  });
}

for (const period of ["historical", "current-term", "current-year"] as const) {
  test(`SSR adapters forward ${period} and reject mismatched scope`, async () => {
    const urls: string[] = [];
    const fetchDetails: typeof fetch = async (input) => {
      urls.push(String(input));
      return Response.json({ ...memberDetails, period });
    };
    const route = { kind: "member", id: "1", view: "summary" } as const;
    const result = await loadDetails(
      route,
      signal(),
      fetchDetails,
      "http://api",
      period,
    );
    expect(result.kind).toBe("member");
    expect(urls).toEqual([`http://api/members/1?period=${period}`]);
    const mismatched = period === "historical" ? "current-term" : "historical";
    expect(
      (
        await loadDetails(
          route,
          signal(),
          reply({ ...memberDetails, period: mismatched }),
          "http://api",
          period,
        )
      ).kind,
    ).toBe("error");
    await expect(
      loadRankings(
        reply({ ...rankingFixture, period }),
        signal(),
        "http://api",
        period,
      ),
    ).resolves.toHaveProperty("period", period);
    await expect(
      loadRankings(
        reply({ ...rankingFixture, period: mismatched }),
        signal(),
        "http://api",
        period,
      ),
    ).rejects.toThrow("unexpected response");
  });
}
for (const period of [undefined, null, "", "other"]) {
  test(`detail and ranking parsers require recognized scope ${String(period)}`, async () => {
    expect(
      (
        await loadDetails(
          funderRoute,
          signal(),
          reply({ ...funderDetails, period }),
          "http://api",
        )
      ).kind,
    ).toBe("error");
    expect(() => parseRankings({ ...rankingFixture, period })).toThrow(
      "unexpected response",
    );
  });
}

test("rankings accept complete boards and validate identities beyond the first batch", () => {
  const fixture = completeRankingFixture();
  const parsed = parseRankings(fixture);
  for (const category of [
    "members",
    "individuals",
    "companies",
    "parties",
  ] as const) {
    expect(parsed[category]).toHaveLength(23);
    expect(parsed[category].map((row) => row.id)).toEqual(
      fixture[category].map((row) => row.id),
    );
    expect(() =>
      parseRankings({
        ...fixture,
        [category]: [...fixture[category], fixture[category][0]],
      }),
    ).toThrow();
    expect(() =>
      parseRankings({
        ...fixture,
        [category]: [
          ...fixture[category],
          { ...fixture[category][0], id: "bad" },
        ],
      }),
    ).toThrow();
  }
});

test("ranking bar scale includes negative extrema beyond the initial prefix", () => {
  const fixture = completeRankingFixture();
  fixture.members[0].total.amount = "10";
  fixture.members[22].total.amount = "-100";
  const rows = rankingRows(parseRankings(fixture), "members");
  expect(rows[0].width).toBe(10);
  expect(rows[22].width).toBe(100);
  expect(rows[22].negative).toBe(true);
});
