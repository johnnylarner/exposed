import {
  memberIdPattern,
  type DeclaredGbpPeriodTotal,
} from "../entity-details";
import type { Overview, OverviewState, Quartile } from "../overview";
function malformed(): never {
  throw new Error("Unexpected overview response");
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
function integer(value: unknown): number {
  return typeof value === "number" && Number.isSafeInteger(value)
    ? value
    : malformed();
}
function count(value: unknown): number {
  const n = integer(value);
  return n >= 0 ? n : malformed();
}
function bool(value: unknown): boolean {
  return typeof value === "boolean" ? value : malformed();
}
function list(value: unknown): unknown[] {
  return Array.isArray(value) ? value : malformed();
}
function decimal(value: unknown): string | null {
  return value === null
    ? null
    : typeof value === "string" && /^-?\d+(?:\.\d+)?$/u.test(value)
      ? value
      : malformed();
}
function timestamp(value: unknown): string {
  const result = text(value);
  return Number.isFinite(Date.parse(result)) ? result : malformed();
}
function quartile(value: unknown): Quartile {
  if (value === 1 || value === 2 || value === 3 || value === 4) return value;
  return malformed();
}
function total(value: unknown): DeclaredGbpPeriodTotal {
  const row = record(value);
  return {
    amount: decimal(row.amount),
    declarationCount: count(row.declaration_count),
    incomplete: bool(row.incomplete),
  };
}
export function parseOverview(value: unknown): Overview {
  const row = record(value);
  const quartiles = list(row.quartiles).map((value) => {
    const q = record(value);
    return {
      quartile: quartile(q.quartile),
      memberCount: count(q.member_count),
      minimum: decimal(q.minimum),
      maximum: decimal(q.maximum),
      incompleteMemberCount: count(q.incomplete_member_count),
    };
  });
  if (
    quartiles.length !== 4 ||
    quartiles.some(
      (q, i) =>
        q.quartile !== i + 1 ||
        q.incompleteMemberCount > q.memberCount ||
        (q.memberCount === 0
          ? q.minimum !== null || q.maximum !== null
          : q.minimum === null || q.maximum === null),
    )
  )
    return malformed();
  const memberCount = count(row.member_count);
  const unavailableMemberCount = count(row.unavailable_member_count);
  if (
    quartiles.reduce((sum, q) => sum + q.memberCount, 0) +
      unavailableMemberCount !==
    memberCount
  )
    return malformed();
  return {
    currentTermStart: timestamp(row.current_term_start),
    memberCount,
    unavailableMemberCount,
    incomplete: bool(row.incomplete),
    quartiles,
    topMembers: list(row.top_members).map((value) => {
      const r = record(value);
      const member = record(r.member);
      const id = text(member.id);
      if (
        !memberIdPattern.test(id) ||
        Number(id) > 2147483647 ||
        member.is_current_commons !== true
      )
        return malformed();
      const declared = total(r.total);
      if (declared.amount === null) return malformed();
      return {
        member: {
          id,
          name: text(member.name),
          partyName: text(member.party_name),
          membershipFrom: text(member.membership_from),
          isCurrentCommons: true,
        },
        total: declared,
      };
    }),
    parties: list(row.parties).map((value) => {
      const r = record(value);
      return {
        partyId: integer(r.party_id),
        partyName: text(r.party_name),
        total: total(r.total),
        memberCount: count(r.member_count),
      };
    }),
  };
}
export async function loadOverview(
  signal: AbortSignal,
  fetch: typeof globalThis.fetch,
  origin: string,
): Promise<OverviewState> {
  try {
    const response = await fetch(`${origin}/overview`, {
      signal: AbortSignal.any([signal, AbortSignal.timeout(10000)]),
    });
    if (!response.ok) return { kind: "unavailable" };
    return {
      kind: "available",
      overview: parseOverview(await response.json()),
    };
  } catch (error) {
    if (signal.aborted) throw error;
    return { kind: "unavailable" };
  }
}
