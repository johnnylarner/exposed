#!/usr/bin/env node
import { unflatten } from "devalue";
import { mkdir, writeFile, readFile, appendFile } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { tmpdir } from "node:os";
import { randomUUID } from "node:crypto";

const fail = (message) => {
  throw new Error(message);
};
const record = (v) =>
  v && typeof v === "object" && !Array.isArray(v) ? v : fail("Expected object");
const string = (v) => (typeof v === "string" ? v : fail("Expected string"));
const nonempty = (v) => (string(v).trim() ? v : fail("Expected nonempty text"));
const timestamp = (v) =>
  Number.isFinite(Date.parse(string(v))) ? v : fail("Invalid timestamp");
const nullableTimestamp = (v) => (v === null ? null : timestamp(v));
const nullable = (v) => (v === null ? null : string(v));
const count = (v) =>
  Number.isSafeInteger(v) && v >= 0 ? v : fail("Expected nonnegative count");
const integer = (v) => (Number.isSafeInteger(v) ? v : fail("Expected integer"));
const bool = (v) => (typeof v === "boolean" ? v : fail("Expected boolean"));
const array = (v, parse) =>
  Array.isArray(v) ? v.map(parse) : fail("Expected array");
const decimal = (v) =>
  v === null || (typeof v === "string" && /^-?\d+(?:\.\d+)?$/u.test(v))
    ? v
    : fail("Expected exact decimal string or null");
const id = (v, kind) =>
  (kind === "member"
    ? /^[1-9]\d*$/u
    : /^[\da-f]{8}(?:-[\da-f]{4}){3}-[\da-f]{12}$/iu
  ).test(string(v))
    ? v
    : fail("Invalid identity");
const total = (v) => {
  record(v);
  return {
    amount: decimal(v.amount),
    entryCount: count(v.entryCount),
    unknownAmountCount: count(v.unknownAmountCount),
  };
};
const member = (v) => {
  record(v);
  return {
    id: id(v.id, "member"),
    name: string(v.name),
    partyName: string(v.partyName),
    membershipFrom: string(v.membershipFrom),
    isCurrentCommons: bool(v.isCurrentCommons),
  };
};
export function decode(body) {
  const envelope = record(JSON.parse(body));
  if (envelope.type !== "data") fail("Expected SvelteKit data envelope");
  const nodes = array(envelope.nodes, (v) => v);
  const models = nodes
    .filter((v) => v?.type === "data")
    .map((v) => unflatten(v.data))
    .filter((v) => v?.model)
    .map((v) => v.model);
  if (models.length !== 1) fail("Expected exactly one page model");
  return record(models[0]);
}
export function search(model, query) {
  if (
    model.kind !== "search" ||
    model.query !== query ||
    model.result?.status !== "success" ||
    model.result.term !== query
  )
    fail("Search model/status/query mismatch");
  if (!Array.isArray(model.result.entities) || !model.result.entities.length)
    fail("Expected nonempty selected search results");
  return array(model.result.entities, (v) => {
    record(v);
    if (!["MP", "Funder"].includes(v.kind)) fail("Unknown entity kind");
    id(v.id, v.kind === "MP" ? "member" : "funder");
    const source = record(v.matchSource);
    const matchSource =
      source.kind === "name"
        ? { kind: "name" }
        : source.kind === "alias"
          ? { kind: "alias", name: nonempty(source.name) }
          : fail("Unknown provenance");
    const funderKind = nullable(v.funderKind);
    if (v.kind === "MP" && (funderKind !== null || source.kind !== "name"))
      fail("Invalid MP context");
    return {
      ...(v.kind === "MP" ? { id: v.id } : {}),
      name: string(v.name),
      kind: v.kind,
      funderKind,
      matchSource,
    };
  });
}
export function selectFunder(model, selector) {
  search(model, selector.query);
  const rows = model.result.entities.filter(
    (v) =>
      v.kind === "Funder" &&
      v.name === selector.name &&
      v.funderKind === selector.funderKind,
  );
  if (rows.length !== 1)
    fail(`Identity selector matched ${rows.length} funders: ${selector.name}`);
  return rows[0].id;
}
export function details(model, selector, selectedId) {
  if (
    model.kind !== "detail" ||
    model.route?.kind !== selector.kind ||
    model.route.id !== selectedId ||
    model.route.view !== "summary" ||
    model.result?.kind !== selector.kind
  )
    fail("Detail route/result mismatch");
  const d = record(model.result.details);
  if (selector.kind === "member") {
    const profile = member(d.member);
    if (profile.id !== selectedId || profile.name !== selector.name)
      fail("Selected member identity mismatch");
    const s = record(d.summary),
      t = record(s.declaredTotals);
    const period = (v) => {
      record(v);
      return {
        amount: decimal(v.amount),
        declarationCount: count(v.declarationCount),
        incomplete: bool(v.incomplete),
      };
    };
    return {
      member: profile,
      summary: {
        declaredTotals: {
          currentTermStart: timestamp(t.currentTermStart),
          currentTerm: period(t.currentTerm),
          historical: period(t.historical),
        },
        entryCount: count(s.entryCount),
        unknownCurrencyCount: count(s.unknownCurrencyCount),
        undatedDeclarationCount: count(s.undatedDeclarationCount),
        firstRegisteredAt: nullableTimestamp(s.firstRegisteredAt),
        lastRegisteredAt: nullableTimestamp(s.lastRegisteredAt),
        currencies: array(s.currencies, (v) => ({
          ...total(v),
          currency: string(v.currency),
          categories: array(v.categories, (c) => ({
            ...total(c),
            categoryId: integer(c.categoryId),
            categoryName: string(c.categoryName),
          })),
        })),
      },
    };
  }
  const f = record(d.funder);
  if (
    id(f.id, "funder") !== selectedId ||
    f.name !== selector.name ||
    f.funderKind !== selector.funderKind
  )
    fail("Selected funder identity mismatch");
  return {
    funder: {
      name: string(f.name),
      funderKind: nullable(f.funderKind),
      companyNumber: nullable(f.companyNumber),
      aliases: array(f.aliases, string),
    },
    entryCount: count(d.entryCount),
    unknownCurrencyCount: count(d.unknownCurrencyCount),
    recipientLimit: count(d.recipientLimit),
    currencies: array(d.currencies, (v) => ({
      currency: string(v.currency),
      amount: decimal(v.amount),
      parties: array(v.parties, (p) => ({
        ...total(p),
        partyName: string(p.partyName),
      })),
      topRecipients: array(v.topRecipients, (r) => ({
        ...total(r),
        member: member(r.member),
      })),
    })),
  };
}
export function differences(a, b, path = "$") {
  if (Object.is(a, b)) return [];
  if (
    a === null ||
    b === null ||
    typeof a !== "object" ||
    typeof b !== "object" ||
    Array.isArray(a) !== Array.isArray(b)
  )
    return [{ path, production: a, development: b }];
  return [...new Set([...Object.keys(a), ...Object.keys(b)])].flatMap((k) =>
    differences(a[k], b[k], `${path}.${k}`),
  );
}
function origin(value) {
  const u = new URL(value);
  if (
    !["http:", "https:"].includes(u.protocol) ||
    u.username ||
    u.password ||
    u.pathname !== "/" ||
    u.search ||
    u.hash
  )
    fail("Expected plain HTTP origin");
  return u.origin;
}
export function cases(value) {
  record(value);
  const searches = array(value.searches, nonempty);
  const entities = array(value.entities, (v) => {
    record(v);
    nonempty(v.name);
    if (v.kind === "member") id(v.id, "member");
    else if (v.kind === "funder") {
      nonempty(v.query);
      nullable(v.funderKind);
    } else fail("Invalid selector kind");
    return v;
  });
  if (!searches.length || !entities.length) fail("Empty selection");
  return { searches, entities };
}
export async function run(options) {
  const production = origin(options.production ?? "https://exposed.today"),
    development = origin(options.development);
  if (production === development)
    fail("Production and development origins must differ");
  const selection = cases(options.cases);
  const evidence = resolve(
    options.evidence ?? `${tmpdir()}/exposed-live-${randomUUID()}`,
  );
  await mkdir(evidence);
  console.log(`Evidence: ${evidence}`);
  const save = (name, value) =>
    writeFile(`${evidence}/${name}`, JSON.stringify(value, null, 2) + "\n");
  const report = {
    startedAt: new Date().toISOString(),
    production,
    development,
    status: "incomplete",
    outcomes: [],
  };
  await save("report.json", report);
  let sequence = 0;
  async function get(base, path) {
    const name = String(++sequence).padStart(3, "0"),
      request = {
        url: base + path,
        method: "GET",
        startedAt: new Date().toISOString(),
      };
    await save(`${name}.request.json`, request);
    try {
      const response = await fetch(request.url, {
        method: "GET",
        redirect: "manual",
        signal: AbortSignal.timeout(options.timeoutMs ?? 15000),
      });
      request.status = response.status;
      request.headers = Object.fromEntries(response.headers);
      const chunks = [];
      await writeFile(`${evidence}/${name}.body.txt`, "");
      let size = 0;
      for await (const chunk of response.body) {
        await appendFile(`${evidence}/${name}.body.txt`, chunk);
        size += chunk.length;
        if (size > 8 * 1024 * 1024) {
          fail("Response exceeds 8 MiB");
        }
        chunks.push(chunk);
      }
      const body = Buffer.concat(chunks).toString("utf8");
      await writeFile(`${evidence}/${name}.body.txt`, body);
      if (!response.ok) fail(`HTTP ${response.status}`);
      return decode(body);
    } catch (error) {
      request.error = String(error);
      throw error;
    } finally {
      request.finishedAt = new Date().toISOString();
      await save(`${name}.request.json`, request);
    }
  }
  async function observe(base, spec) {
    if (spec.kind === "search")
      return search(
        await get(
          base,
          `/search/__data.json?q=${encodeURIComponent(spec.query)}`,
        ),
        spec.query,
      );
    const selectedId =
      spec.kind === "member"
        ? spec.id
        : selectFunder(
            await get(
              base,
              `/search/__data.json?q=${encodeURIComponent(spec.query)}`,
            ),
            spec,
          );
    return details(
      await get(
        base,
        `/${spec.kind === "member" ? "members" : "funders"}/${selectedId}/__data.json`,
      ),
      spec,
      selectedId,
    );
  }
  for (const [index, spec] of [
    ...selection.searches.map((query) => ({ kind: "search", query })),
    ...selection.entities,
  ].entries()) {
    const outcome = { case: spec, observations: {}, errors: {} };
    for (const [label, base] of [
      ["production", production],
      ["development", development],
    ]) {
      try {
        outcome.observations[label] = await observe(base, spec);
      } catch (error) {
        outcome.errors[label] = String(error);
      }
      await save(`case-${index}.json`, outcome);
    }
    outcome.diffs = Object.keys(outcome.errors).length
      ? []
      : differences(
          outcome.observations.production,
          outcome.observations.development,
        );
    outcome.status = Object.keys(outcome.errors).length
      ? "error"
      : outcome.diffs.length
        ? "different"
        : "matched";
    report.outcomes.push(outcome);
    await save(`case-${index}.json`, outcome);
    await save("report.json", report);
    console.log(`${outcome.status.toUpperCase()} ${spec.query ?? spec.name}`);
  }
  report.status = report.outcomes.some((v) => v.status === "error")
    ? "error"
    : report.outcomes.some((v) => v.status === "different")
      ? "different"
      : "matched";
  report.finishedAt = new Date().toISOString();
  await save("report.json", report);
  await writeFile(
    `${evidence}/diff.txt`,
    report.outcomes
      .map(
        (v) =>
          `${v.status}: ${JSON.stringify(v.case)}\n${JSON.stringify(v.errors)}\n${v.diffs.map((d) => `${d.path}: production=${JSON.stringify(d.production)} development=${JSON.stringify(d.development)}`).join("\n")}`,
      )
      .join("\n\n") + "\n",
  );
  return {
    code: report.status === "error" ? 2 : report.status === "different" ? 1 : 0,
    evidence,
    report,
  };
}
if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  try {
    const args = process.argv.slice(2),
      options = {};
    if (args.length === 1 && args[0] === "--help") {
      console.log(
        "Usage: compare-live.mjs --development ORIGIN [--production ORIGIN] [--cases PATH] [--evidence NEW_DIRECTORY]",
      );
      process.exit(0);
    }
    for (let i = 0; i < args.length; i += 2) {
      if (
        !["--development", "--production", "--cases", "--evidence"].includes(
          args[i],
        ) ||
        !args[i + 1]
      )
        fail(
          "Usage: compare-live.mjs --development ORIGIN [--production ORIGIN] [--cases PATH] [--evidence NEW_DIRECTORY]",
        );
      options[args[i].slice(2)] = args[i + 1];
    }
    options.cases = JSON.parse(
      await readFile(
        options.cases ?? new URL("../live-cases.json", import.meta.url),
        "utf8",
      ),
    );
    process.exitCode = (await run(options)).code;
  } catch (error) {
    console.error(String(error));
    process.exitCode = 2;
  }
}
