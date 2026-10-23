import { expect, test } from "@playwright/test";
import { searchEntities } from "../src/lib/server/search";
import { loadDetails } from "../src/lib/server/entity-details";
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
  expect(valid.details.currencies[1].parties[0].amount).toBe(
    "18014398509481986.246913578",
  );
  const body = {
    ...funderDetails,
    currencies: funderDetails.currencies.map((group, index) =>
      index === 1
        ? {
            ...group,
            parties: group.parties.map((party, partyIndex) =>
              partyIndex === 0 ? { ...party, amount: 9007199254740993 } : party,
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
      "18014398509481986.246913578",
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
