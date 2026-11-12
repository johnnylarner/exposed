import { expect, test } from "@playwright/test";
import { searchEntities } from "../src/lib/server/search";

import { loadLords, parseLordsDetails } from "../src/lib/server/lords";
import {
  currentLordsCategories,
  groupLordsCategories,
  sourceCalendarDate,
  type RegisteredInterest,
} from "../src/lib/lords";
import { shareholdingSummary } from "../src/lib/lords-shareholdings";
import { lordHref, readDetailRoute } from "../src/lib/entity-details";
import { entityHref } from "../src/lib/search";
import { lordsFixture, sourceInterest } from "./lords-fixtures";

function reply(body: unknown): typeof fetch {
  return async () => Response.json(body);
}
const signal = () => new AbortController().signal;

for (const [source, labels] of [
  ["Blend Media Limited (media)", ["Blend Media Limited"]],
  [
    "Visionable Limited (formerly IOCOM UK Ltd) (visual business platform)",
    ["Visionable Limited"],
  ],
  [
    "50 per cent holding in TomahawkPro Ltd (collaborative software & IT innovation) (interest ceased 31 December 2016)",
    ["TomahawkPro Ltd"],
  ],
  [
    "100 per cent ownership with partner of Leadership in Mind Ltd (coaching and consultancy)",
    ["Leadership in Mind Ltd"],
  ],
  [
    "Sole shareholder, Just Resolutions Limited (Member's private company, not trading at present)",
    ["Just Resolutions Limited"],
  ],
  [
    "Shareholding in Centre for Social Research Ltd (interest ceased 27 July 2016)",
    ["Centre for Social Research Ltd"],
  ],
  [
    "ARTIS (Europe) Ltd (research and risk modelling, including psychology of terrorism and political violence)",
    ["ARTIS (Europe) Ltd"],
  ],
  ["eToro (UK) Limited (financial services)", ["eToro (UK) Limited"]],
  [
    "Boot (Henry) plc (construction and property development) (interest ceased 5 April 2023)",
    ["Boot (Henry) plc"],
  ],
  [
    "Automatic Data Processing, Inc. (human resources data management) (interest ceased 1 April 2026)",
    ["Automatic Data Processing, Inc."],
  ],
  [
    "Project Alpha Topco Limited, Project Alpha Midco Limited and Project Alpha Bidco Limited (parent companies of Access Education Group (formerly Access Creative College), the independent provider of specialist training, apprenticeships and further education)",
    [
      "Project Alpha Topco Limited",
      "Project Alpha Midco Limited",
      "Project Alpha Bidco Limited",
    ],
  ],
  [
    "Daily Mail and General Trust plc (media)\r\nInforma plc (media) (interest ceased 3 February 2019)",
    ["Daily Mail and General Trust plc", "Informa plc"],
  ],
  ["Acme Ltd (first); Acme Ltd (second)", ["Acme Ltd", "Acme Ltd"]],
] satisfies [string, string[]][]) {
  test(`shareholding labels retain every supported name in ${source}`, () => {
    expect(shareholdingSummary(source)).toEqual({
      kind: "labels",
      text: labels.join("; "),
    });
  });
}

for (const source of [
  "Warburg Pincus No X (international private equity fund)",
  "HSBC Vision 2020, 2021, 2022, 2023, 2024 (discretionary investment funds)",
  "2% Canadian Government Bond Snr 01 Dec 14 (government bond)",
  "Air Asia (Bahrain) (airline)",
  "The Member and his wife are majority shareholders of Global HDE Ltd (consultants in Health, Design and Education - see category 1))",
  "The member is a shareholder with a controlling interest in a property management company; full details are held by the Registrar of Lords’ Interests",
  "All shareholdings are managed by Rathbones and the member has no influence over any of the dealings although the following registrable shareholdings are held:",
  "My listed equity portfolio detailed below is held by Lombard Odier Darier Hentsch in Geneva, Switzerland. Abbott Laboratories Limited (pharmaceuticals); Allianz SE (financial services)",
  "SSAP Pension Fund, holding shares in excess of £50,000 in:\nBP plc (global oil and gas company)\nBTG plc (pharmaceuticals)",
  "Acme Ltd (services) and an unnamed company",
  "Acme Ltd (services) but shares are held by a manager",
  "Acme Ltd (unclosed description",
  "Acme Ltd,",
  "Unknown Company",
  '<img src=x onerror="alert(1)"> & Partners\nSecond line',
]) {
  test(`shareholding preview retains unsupported source ${source}`, () => {
    const summary = shareholdingSummary(source);
    expect(summary.kind).toBe("source-preview");
    expect(summary.text.length).toBeLessThanOrEqual(113);
    expect(summary.text).toEqual(
      source.length <= 112
        ? source.replace(/\s+/gu, " ")
        : expect.stringMatching(/…$/u),
    );
  });
}

test("shareholding summaries distinguish missing text and bound source previews", () => {
  for (const source of [null, "", " \n\t "]) {
    expect(shareholdingSummary(source)).toEqual({
      kind: "missing",
      text: "Interest text unavailable",
    });
  }
  expect(shareholdingSummary("Acme Ltd ".repeat(2000)).kind).toBe(
    "source-preview",
  );
  expect(shareholdingSummary("X".repeat(150)).text).toBe(`${"X".repeat(112)}…`);
});

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
    "ARTIS (Europe) Ltd (research and risk modelling)",
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
  expect(readDetailRoute("/lords/2175")).toEqual({
    kind: "lord",
    id: "2175",
    view: "summary",
  });
  expect(readDetailRoute("/lords/2175/history/")).toEqual({
    kind: "lord",
    id: "2175",
    view: "history",
  });
  expect(
    lordHref("2175", "history", new URLSearchParams("q=A%26B&period=invalid")),
  ).toBe("/lords/2175/history?q=A%26B");
  expect(lordHref("2175", "summary", new URLSearchParams())).toBe(
    "/lords/2175",
  );
  for (const path of [
    "/lords/0",
    "/lords/2147483648/history",
    "/lords/no/history",
    "/lords/2175/other",
    "/lords/2175/history/extra",
  ]) {
    expect(readDetailRoute(path)).toEqual({ kind: "invalid" });
  }
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

test("current Lords summary retains nine active occurrences without mutating the twelve source occurrences", () => {
  const details = parseLordsDetails(lordsFixture);
  const original = structuredClone(details);
  const summary = currentLordsCategories(details.categories);
  function flatten(interests: RegisteredInterest[]): RegisteredInterest[] {
    return interests.flatMap((interest) => [
      interest,
      ...flatten(interest.children),
    ]);
  }
  const all = details.categories.flatMap((category) =>
    flatten(category.interests),
  );
  const active = summary.flatMap((category) => flatten(category.interests));
  expect(all).toHaveLength(12);
  expect(active).toHaveLength(9);
  expect(active.every((interest) => interest.deletedWhen === null)).toBe(true);
  expect(
    active.filter((interest) => interest.text === "Repeated holding"),
  ).toHaveLength(2);
  expect(new Set(active.map((interest) => interest.sourceId)).size).toBe(1);
  expect(summary[0].interests.map((interest) => interest.text)).toEqual([
    "Active parent",
    "Active child",
    "Interest ceased on 1 January 2026",
  ]);
  expect(details).toEqual(original);
  expect(summary[0]).not.toBe(details.categories[0]);
  expect(summary[0].interests[0]).not.toBe(details.categories[0].interests[0]);
});

test("summary promotes across multiple deleted ancestors while preserving active nesting, source order and duplicate occurrences", () => {
  const category = parseLordsDetails(lordsFixture).categories[0];
  const active = category.interests[0];
  const deleted = category.interests[1];
  const survivor = {
    ...active,
    text: "survivor",
    children: [{ ...active, text: "nested", children: [] }],
  };
  const source = [
    {
      ...category,
      interests: [
        { ...active, text: "before", children: [] },
        {
          ...deleted,
          children: [{ ...deleted, children: [survivor, survivor] }],
        },
        { ...active, text: "after", children: [] },
      ],
    },
    {
      ...category,
      name: "deleted only",
      interests: [{ ...deleted, children: [] }],
    },
    { ...category, interests: [] },
  ];
  const original = structuredClone(source);
  const summary = currentLordsCategories(source);
  expect(summary).toHaveLength(1);
  expect(summary[0].interests.map((interest) => interest.text)).toEqual([
    "before",
    "survivor",
    "survivor",
    "after",
  ]);
  expect(
    summary[0].interests[1].children.map((interest) => interest.text),
  ).toEqual(["nested"]);
  expect(source).toEqual(original);
});
