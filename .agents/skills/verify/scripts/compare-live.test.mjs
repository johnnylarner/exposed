import { test } from "node:test";
import assert from "node:assert/strict";
import { createServer } from "node:http";
import { stringify } from "devalue";
import { mkdtemp, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import {
  cases,
  decode,
  search,
  details,
  selectFunder,
  differences,
  run,
} from "./compare-live.mjs";
const uuid = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
  other = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";
const selector = {
  kind: "funder",
  query: "bbc",
  name: "BBC",
  funderKind: "Company",
};
const row = (id) => ({
  id,
  name: "BBC",
  kind: "Funder",
  funderKind: "Company",
  matchSource: { kind: "name" },
});
const searchModel = (entities) => ({
  kind: "search",
  query: "bbc",
  result: { status: "success", term: "bbc", entities },
});
const envelope = (model) =>
  JSON.stringify({
    type: "data",
    nodes: [null, { type: "data", data: JSON.parse(stringify({ model })) }],
  });
const total = {
  amount: "9007199254740993.01",
  entryCount: 2,
  unknownAmountCount: 0,
};
const profile = {
  id: "178",
  name: "John McDonnell",
  partyName: "Independent",
  membershipFrom: "Hayes and Harlington",
  isCurrentCommons: true,
};
const funder = (id) => ({
  kind: "detail",
  route: { kind: "funder", id, view: "summary" },
  result: {
    kind: "funder",
    details: {
      funder: {
        id,
        name: "BBC",
        funderKind: "Company",
        companyNumber: null,
        aliases: [],
      },
      entryCount: 2,
      unknownCurrencyCount: 0,
      recipientLimit: 10,
      currencies: [
        {
          currency: "GBP",
          amount: total.amount,
          parties: [{ ...total, partyName: "Independent" }],
          topRecipients: [{ ...total, member: profile }],
        },
      ],
    },
  },
});
const mp = { kind: "member", id: "178", name: "John McDonnell" };
const member = () => ({
  kind: "detail",
  route: { kind: "member", id: "178", view: "summary" },
  result: {
    kind: "member",
    details: {
      member: profile,
      summary: {
        declaredTotals: {
          currentTermStart: "2024-07-09T00:00:00Z",
          currentTerm: {
            amount: total.amount,
            declarationCount: 2,
            incomplete: false,
          },
          historical: { amount: null, declarationCount: 3, incomplete: true },
        },
        entryCount: 3,
        unknownCurrencyCount: 1,
        undatedDeclarationCount: 1,
        firstRegisteredAt: null,
        lastRegisteredAt: null,
        currencies: [
          {
            ...total,
            currency: "GBP",
            categories: [
              { ...total, categoryId: 1, categoryName: "Employment" },
            ],
          },
        ],
      },
    },
  },
});
test("real-shaped envelope preserves ordered search, multiplicity, provenance and context", () => {
  const a = search(
    decode(
      envelope(searchModel([row(uuid), { ...row(other), name: "Other" }])),
    ),
    "bbc",
  );
  for (const entities of [
    [{ ...row(other), name: "Other" }, row(uuid)],
    [row(uuid), row(uuid)],
    [
      {
        ...row(uuid),
        matchSource: {
          kind: "alias",
          name: "British Broadcasting Corporation",
        },
      },
      { ...row(other), name: "Other" },
    ],
    [
      { ...row(uuid), funderKind: null },
      { ...row(other), name: "Other" },
    ],
  ])
    assert.ok(differences(a, search(searchModel(entities), "bbc")).length);
  assert.throws(() =>
    search(
      { kind: "search", query: "bbc", result: { status: "error" } },
      "bbc",
    ),
  );
  assert.throws(() => decode('{"type":"redirect"}'));
});
test("independent UUID selection and asserted detail identity", () => {
  assert.equal(selectFunder(searchModel([row(other)]), selector), other);
  assert.deepEqual(
    details(funder(uuid), selector, uuid),
    details(funder(other), selector, other),
  );
  assert.throws(() =>
    selectFunder(searchModel([row(uuid), row(other)]), selector),
  );
  assert.throws(() => selectFunder(searchModel([]), selector));
  const wrong = funder(uuid);
  wrong.result.details.funder.name = "Other";
  assert.throws(() => details(wrong, selector, uuid));
  assert.throws(() => details(funder(other), selector, uuid));
});
test("exact financial mutation controls cover null, counts, completeness and every breakdown", () => {
  const original = member(),
    expected = details(original, mp, "178");
  for (const mutate of [
    (v) =>
      (v.summary.declaredTotals.currentTerm.amount = "9007199254740993.02"),
    (v) => (v.summary.declaredTotals.historical.amount = "0"),
    (v) => v.summary.declaredTotals.currentTerm.declarationCount++,
    (v) => (v.summary.declaredTotals.currentTerm.incomplete = true),
    (v) => v.summary.unknownCurrencyCount++,
    (v) => v.summary.undatedDeclarationCount++,
    (v) => (v.summary.currencies[0].categories[0].amount = "0"),
  ]) {
    const v = structuredClone(original);
    mutate(v.result.details);
    assert.ok(differences(expected, details(v, mp, "178")).length);
  }
  const missing = member();
  delete missing.result.details.summary.declaredTotals;
  assert.throws(() => details(missing, mp, "178"));
  const numeric = member();
  numeric.result.details.summary.declaredTotals.currentTerm.amount = 123;
  assert.throws(() => details(numeric, mp, "178"));
  const f = funder(uuid),
    base = details(f, selector, uuid);
  for (const mutate of [
    (d) => d.recipientLimit++,
    (d) => (d.currencies[0].parties[0].amount = null),
    (d) => (d.currencies[0].topRecipients[0].member.id = "4613"),
    (d) => d.currencies[0].topRecipients[0].unknownAmountCount++,
  ]) {
    const v = structuredClone(f);
    mutate(v.result.details);
    assert.ok(differences(base, details(v, selector, uuid)).length);
  }
});
async function server(handler) {
  const s = createServer(handler);
  await new Promise((r) => s.listen(0, "127.0.0.1", r));
  return {
    origin: `http://127.0.0.1:${s.address().port}`,
    close: () => new Promise((r) => s.close(r)),
  };
}
test("HTTP replay compares separately resolved IDs and retains evidence", async () => {
  const serve = (id) => (req, res) =>
    res.end(
      envelope(
        req.url.startsWith("/search") ? searchModel([row(id)]) : funder(id),
      ),
    );
  const p = await server(serve(uuid)),
    d = await server(serve(other));
  try {
    const root = await mkdtemp(`${tmpdir()}/verify-test-`);
    const result = await run({
      production: p.origin,
      development: d.origin,
      cases: { searches: ["bbc"], entities: [selector] },
      evidence: `${root}/run`,
    });
    assert.equal(result.code, 0);
    assert.equal(
      JSON.parse(await readFile(`${root}/run/report.json`)).status,
      "matched",
    );
    await assert.rejects(() =>
      run({
        production: p.origin,
        development: p.origin,
        cases: { searches: ["bbc"], entities: [selector] },
      }),
    );
  } finally {
    await p.close();
    await d.close();
  }
});
test("HTTP failures, redirects, malformed data and timeout never match, even both errors", async () => {
  for (const mode of ["http", "redirect", "malformed", "timeout"]) {
    const handler = (req, res) => {
      if (mode === "timeout") {
        setTimeout(() => res.end("{}"), 80);
        return;
      }
      res.statusCode = mode === "http" ? 503 : mode === "redirect" ? 302 : 200;
      res.end("{}");
    };
    const p = await server(handler),
      d = await server(handler);
    try {
      const root = await mkdtemp(`${tmpdir()}/verify-error-`);
      const result = await run({
        production: p.origin,
        development: d.origin,
        cases: { searches: ["bbc"], entities: [selector] },
        evidence: `${root}/run`,
        timeoutMs: 20,
      });
      assert.equal(result.code, 2);
      assert.ok(result.report.outcomes.every((v) => v.status === "error"));
      assert.ok(
        (await readFile(`${root}/run/001.request.json`, "utf8")).includes(
          "error",
        ),
      );
    } finally {
      await p.close();
      await d.close();
    }
  }
});

test("selected nonempty queries/results and valid financial dates are required", () => {
  assert.throws(() => search(searchModel([]), "bbc"));
  assert.throws(() => cases({ searches: ["   "], entities: [selector] }));
  assert.throws(() =>
    cases({ searches: ["bbc"], entities: [{ ...selector, query: "" }] }),
  );
  const emptyAlias = row(uuid);
  emptyAlias.matchSource = { kind: "alias", name: "   " };
  assert.throws(() => search(searchModel([emptyAlias]), "bbc"));
  const invalid = member();
  invalid.result.details.summary.declaredTotals.currentTermStart = "invalid";
  assert.throws(() => details(invalid, mp, "178"));
});
