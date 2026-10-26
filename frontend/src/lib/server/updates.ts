import { memberIdPattern, funderIdPattern } from "../entity-details";
import type { EntityUpdate, MonthlyUpdates } from "../updates";
function malformed(): never {
  throw new Error(
    "Latest declarations returned an unexpected response. Try again.",
  );
}
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
function record(value: unknown): Record<string, unknown> {
  if (!isRecord(value)) return malformed();
  return value;
}
function text(value: unknown): string {
  return typeof value === "string" ? value : malformed();
}
function timestamp(value: unknown): string {
  const date = text(value);
  if (
    !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?Z$/u.test(date) ||
    !Number.isFinite(Date.parse(date)) ||
    new Date(date).toISOString().slice(0, 19) !== date.slice(0, 19)
  )
    return malformed();
  return date;
}
function entries(
  value: unknown,
  kind: "member" | "company",
  start: number,
  end: number,
): EntityUpdate[] {
  if (!Array.isArray(value)) return malformed();
  const seen = new Set<string>();
  return value.map((value: unknown) => {
    const row = record(value);
    const id = text(row.id);
    if (
      kind === "member"
        ? !memberIdPattern.test(id) || BigInt(id) > 2147483647n
        : !funderIdPattern.test(id)
    )
      return malformed();
    if (seen.has(id.toLowerCase())) return malformed();
    seen.add(id.toLowerCase());
    if (
      typeof row.declaration_count !== "number" ||
      !Number.isSafeInteger(row.declaration_count) ||
      row.declaration_count <= 0
    )
      return malformed();
    const latestRegistration = timestamp(row.latest_registration);
    if (
      Date.parse(latestRegistration) < start ||
      Date.parse(latestRegistration) >= end
    )
      return malformed();
    return {
      id,
      name: text(row.name),
      declarationCount: row.declaration_count,
      latestRegistration,
    };
  });
}
export function parseUpdates(value: unknown): MonthlyUpdates {
  const row = record(value);
  const monthStart = timestamp(row.month_start);
  const monthEnd = timestamp(row.month_end);
  const start = new Date(monthStart);
  if (
    start.getUTCDate() !== 1 ||
    start.getUTCHours() !== 0 ||
    start.getUTCMinutes() !== 0 ||
    start.getUTCSeconds() !== 0 ||
    start.getUTCMilliseconds() !== 0 ||
    /\.\d*[1-9]\d*Z$/u.test(monthStart)
  )
    return malformed();
  const end = new Date(start);
  end.setUTCMonth(end.getUTCMonth() + 1);
  if (
    Date.parse(monthEnd) !== end.getTime() ||
    /\.\d*[1-9]\d*Z$/u.test(monthEnd)
  )
    return malformed();
  return {
    monthStart,
    monthEnd,
    members: entries(row.members, "member", start.getTime(), end.getTime()),
    companies: entries(
      row.companies,
      "company",
      start.getTime(),
      end.getTime(),
    ),
  };
}
export async function loadUpdates(
  fetch: typeof globalThis.fetch,
  signal: AbortSignal,
  origin: string,
): Promise<MonthlyUpdates> {
  const response = await fetch(`${origin}/updates`, {
    signal: AbortSignal.any([signal, AbortSignal.timeout(10_000)]),
  });
  if (!response.ok)
    throw new Error("Latest declarations couldn’t load. Try again.");
  const body: unknown = await response.json();
  return parseUpdates(body);
}
