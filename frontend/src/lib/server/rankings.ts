import { memberIdPattern, funderIdPattern } from "../entity-details";
import type { RankingEntry, Rankings } from "../rankings";
import { isReportingPeriod, type ReportingPeriod } from "../reporting-period";
function malformed(): never {
  throw new Error("Rankings returned an unexpected response. Try again.");
}
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
function record(value: unknown): Record<string, unknown> {
  return isRecord(value) ? value : malformed();
}

function text(value: unknown): string {
  return typeof value === "string" ? value : malformed();
}
function board(
  value: unknown,
  kind: "member" | "funder" | "party",
): RankingEntry[] {
  if (!Array.isArray(value)) return malformed();
  const seen = new Set<string>();
  return value.map((value: unknown) => {
    const row = record(value);
    const id = text(row.id);
    if (
      kind === "member"
        ? !memberIdPattern.test(id) || BigInt(id) > 2147483647n
        : kind === "funder"
          ? !funderIdPattern.test(id)
          : !/^-?\d+$/u.test(id) ||
            BigInt(id) < -2147483648n ||
            BigInt(id) > 2147483647n
    )
      return malformed();
    if (seen.has(id.toLowerCase())) return malformed();
    seen.add(id.toLowerCase());
    const total = record(row.total);
    const amount = total.amount === null ? null : text(total.amount);
    if (amount !== null && !/^-?\d+(?:\.\d+)?$/u.test(amount))
      return malformed();
    if (
      typeof total.declaration_count !== "number" ||
      !Number.isSafeInteger(total.declaration_count) ||
      total.declaration_count < 0 ||
      typeof total.incomplete !== "boolean"
    )
      return malformed();
    return {
      id,
      name: text(row.name),
      context: kind === "member" ? text(row.context) : null,
      total: {
        amount,
        declarationCount: total.declaration_count,
        incomplete: total.incomplete,
      },
    };
  });
}
export function parseRankings(value: unknown): Rankings {
  const row = record(value);
  const period = row.period;
  if (!isReportingPeriod(period)) return malformed();
  const currentTermStart = text(row.current_term_start);
  if (
    !/^\d{4}-\d{2}-\d{2}T.*(?:Z|[+-]\d{2}:\d{2})$/u.test(currentTermStart) ||
    !Number.isFinite(Date.parse(currentTermStart))
  )
    return malformed();
  return {
    period,
    currentTermStart,
    members: board(row.members, "member"),
    individuals: board(row.individuals, "funder"),
    companies: board(row.companies, "funder"),
    parties: board(row.parties, "party"),
  };
}
export async function loadRankings(
  fetch: typeof globalThis.fetch,
  signal: AbortSignal,
  origin: string,
  period: ReportingPeriod = "current-term",
): Promise<Rankings> {
  const response = await fetch(`${origin}/rankings?period=${period}`, {
    signal: AbortSignal.any([signal, AbortSignal.timeout(10_000)]),
  });
  if (!response.ok) throw new Error("Rankings couldn’t load. Try again.");
  const body: unknown = await response.json();
  const result = parseRankings(body);
  if (result.period !== period) return malformed();
  return result;
}
