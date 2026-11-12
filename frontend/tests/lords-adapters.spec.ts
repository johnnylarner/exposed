import { expect, test } from "@playwright/test";
import { searchEntities } from "../src/lib/server/search";

import { loadLords, parseLordsDetails } from "../src/lib/server/lords";
import { groupLordsCategories, sourceCalendarDate } from "../src/lib/lords";
import { readDetailRoute } from "../src/lib/entity-details";
import { entityHref } from "../src/lib/search";
import { lordsFixture, sourceInterest } from "./lords-fixtures";

function reply(body: unknown): typeof fetch {
  return async () => Response.json(body);
}
const signal = () => new AbortController().signal;

test("Lords adapter retains every occurrence, nullable value and source lifecycle timestamp", async () => {
  const parsed = parseLordsDetails(lordsFixture);
  expect(parsed.categories).toHaveLength(5);
  expect(parsed.categories[0].interests[0].children[0].deletedWhen).toBe(
    "2026-11-12T23:45:00-09:00",
  );
  expect(parsed.categories[0].interests[1].children[0].deletedWhen).toBeNull();
  expect(parsed.categories[2].interests[1].text).toBeNull();
  const grouped = groupLordsCategories(parsed.categories);
  expect(grouped.map((category) => category.name)).toEqual([
    null,
    "Category 2: Shareholdings etc. (a)",
    "Category 5: Other interests",
    "Nil",
  ]);
  expect(grouped[1].interests.map((interest) => interest.text)).toEqual([
    "Deleted shareholding",
    "Repeated holding",
    "Repeated holding",
  ]);
  expect(parsed.categories[1].interests).toHaveLength(2);
  expect(sourceCalendarDate("2026-11-12T23:45:00-09:00")).toBe("12 Nov 2026");
  expect(sourceCalendarDate("2025-01-01T01:00:00+02:00")).toBe("1 Jan 2025");
  const urls: string[] = [];
  const result = await loadLords(
    "2175",
    signal(),
    async (input) => {
      urls.push(String(input));
      return Response.json(lordsFixture);
    },
    "http://api",
  );
  expect(result.kind).toBe("lord");
  expect(urls).toEqual(["http://api/lords/2175"]);
});

for (const [name, patch] of [
  ["missing categories", { categories: undefined }],
  ["numeric member ID", { member: { ...lordsFixture.member, id: 2175 } }],
  ["invalid member ID", { member: { ...lordsFixture.member, id: "0" } }],
  [
    "missing nullable metadata",
    { member: { ...lordsFixture.member, party_name: undefined } },
  ],
  ["missing capture date", { captured_at: undefined }],
  ["offset-free capture date", { captured_at: "2026-11-12T12:00:00" }],
  ["invalid capture calendar", { captured_at: "2026-02-30T12:00:00Z" }],
  ...[
    { placement: "unknown" },
    { source_id: 0 },
    { source_id: 1.2 },
    { sort_order: "2" },
    { interests: null },
    ...[
      { text: undefined },
      { source_id: -1 },
      { created_when: "2026-02-29T10:00:00" },
      { created_when: "2026-01-01T25:00:00" },
      { deleted_when: "yesterday" },
      { children: null },
      { children: [{ ...sourceInterest, is_correction: "false" }] },
    ].map((fields) => ({ interests: [{ ...sourceInterest, ...fields }] })),
  ].map((fields): [string, Record<string, unknown>] => [
    JSON.stringify(fields),
    { categories: [{ ...lordsFixture.categories[0], ...fields }] },
  ]),
] satisfies [string, Record<string, unknown>][]) {
  test(`Lords parser rejects ${name}`, () => {
    expect(() => parseLordsDetails({ ...lordsFixture, ...patch })).toThrow(
      "unexpected response",
    );
  });
}

test("Lords loader validates identity and handles missing or unavailable members", async () => {
  expect(
    (await loadLords("1", signal(), reply(lordsFixture), "http://api")).kind,
  ).toBe("error");
  expect(
    await loadLords(
      "2175",
      signal(),
      async () => new Response(null, { status: 404 }),
      "http://api",
    ),
  ).toEqual({ kind: "not-found" });
  expect(
    (
      await loadLords(
        "2175",
        signal(),
        async () => new Response(null, { status: 500 }),
        "http://api",
      )
    ).kind,
  ).toBe("error");
  const nullable = parseLordsDetails({
    ...lordsFixture,
    member: {
      ...lordsFixture.member,
      party_name: null,
      membership_from: null,
      membership_status: null,
    },
    categories: [],
  });
  expect(nullable.categories).toEqual([]);
  expect(nullable.member.partyName).toBeNull();
});

test("Lords search uses its own route and accepts only name provenance", async () => {
  const lord = {
    id: "2175",
    name: "Lord Example",
    kind: "Lords",
    funder_kind: null,
    match_source: { kind: "name" },
  };
  const entities = await searchEntities(
    "Example",
    signal(),
    reply({ entities: [lord] }),
    "http://api",
  );
  expect(entities[0].kind).toBe("Lords");
  expect(entityHref(entities[0], new URLSearchParams("q=Example"))).toBe(
    "/lords/2175?q=Example",
  );
  expect(readDetailRoute("/lords/2175")).toEqual({ kind: "lord", id: "2175" });
  expect(readDetailRoute("/lords/2175/history")).toEqual({ kind: "invalid" });
  for (const fields of [
    { match_source: { kind: "alias", name: "Example" } },
    { funder_kind: "Company" },
    { id: "2147483648" },
  ]) {
    await expect(
      searchEntities(
        "Example",
        signal(),
        reply({ entities: [{ ...lord, ...fields }] }),
        "http://api",
      ),
    ).rejects.toThrow("unexpected Lords identity");
  }
});

test("Lords category grouping distinguishes source names and preserves stable category ties", () => {
  const parsed = parseLordsDetails(lordsFixture);
  const first = parsed.categories[0];
  const second = {
    ...first,
    name: "Second source name",
    interests: [first.interests[0]],
  };
  const grouped = groupLordsCategories([
    first,
    second,
    { ...first, interests: [first.interests[1]] },
  ]);
  expect(grouped.map((category) => category.name)).toEqual([
    first.name,
    "Second source name",
  ]);
  expect(grouped[0].interests).toEqual([
    ...first.interests,
    first.interests[1],
  ]);
  expect(grouped[1].interests).toEqual([first.interests[0]]);
});
