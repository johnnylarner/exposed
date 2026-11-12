import { memberIdPattern } from "../entity-details";
import type {
  LordsCategory,
  LordsDetails,
  LordsState,
  RegisteredInterest,
} from "../lords";

function malformed(): never {
  throw new Error(
    "Registered interests returned an unexpected response. Try again.",
  );
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
function nullableText(value: unknown): string | null {
  return value === null ? null : text(value);
}
function integer(value: unknown): number {
  return typeof value === "number" && Number.isSafeInteger(value)
    ? value
    : malformed();
}
function positiveId(value: unknown): number {
  const id = integer(value);
  return id > 0 ? id : malformed();
}
function list(value: unknown): unknown[] {
  return Array.isArray(value) ? value : malformed();
}
function boolean(value: unknown): boolean {
  return typeof value === "boolean" ? value : malformed();
}
function calendarDate(value: string): boolean {
  const [year, month, day] = value.slice(0, 10).split("-").map(Number);
  const date = new Date(0);
  date.setUTCFullYear(year, month - 1, day);
  date.setUTCHours(0, 0, 0, 0);
  return (
    date.getUTCFullYear() === year &&
    date.getUTCMonth() === month - 1 &&
    date.getUTCDate() === day
  );
}
function timestamp(value: unknown): string | null {
  const result = nullableText(value);
  if (result === null) return null;
  return /^\d{4}-\d{2}-\d{2}T(?:[01]\d|2[0-3]):[0-5]\d:[0-5]\d(?:\.\d+)?(?:Z|[+-](?:[01]\d|2[0-3]):[0-5]\d)?$/u.test(
    result,
  ) && calendarDate(result)
    ? result
    : malformed();
}
function interest(value: unknown): RegisteredInterest {
  const row = record(value);
  return {
    sourceId: positiveId(row.source_id),
    text: nullableText(row.text),
    createdWhen: timestamp(row.created_when),
    lastAmendedWhen: timestamp(row.last_amended_when),
    deletedWhen: timestamp(row.deleted_when),
    isCorrection: boolean(row.is_correction),
    children: list(row.children).map(interest),
  };
}
function category(value: unknown): LordsCategory {
  const row = record(value);
  const placement = row.placement;
  if (placement !== "shareholdings" && placement !== "other")
    return malformed();
  return {
    sourceId: positiveId(row.source_id),
    name: nullableText(row.name),
    sortOrder: integer(row.sort_order),
    placement,
    interests: list(row.interests).map(interest),
  };
}
export function parseLordsDetails(value: unknown): LordsDetails {
  const row = record(value);
  const member = record(row.member);
  const id = text(member.id);
  if (!memberIdPattern.test(id) || Number(id) > 2147483647) return malformed();
  const capturedAt = timestamp(row.captured_at);
  if (capturedAt === null || !/(?:Z|\+00:00)$/u.test(capturedAt))
    return malformed();
  const endDate = timestamp(member.membership_end_date);
  const categories = list(row.categories).map(category);
  const placements = new Map<number, LordsCategory["placement"]>();
  for (const item of categories) {
    const previous = placements.get(item.sourceId);
    if (previous !== undefined && previous !== item.placement)
      return malformed();
    placements.set(item.sourceId, item.placement);
  }
  return {
    member: {
      id,
      name: text(member.name),
      partyName: nullableText(member.party_name),
      membershipFrom: nullableText(member.membership_from),
      membershipStatus: nullableText(member.membership_status),
      membershipEndDate: endDate,
    },
    capturedAt,
    categories,
  };
}

export async function loadLords(
  id: string,
  signal: AbortSignal,
  fetch: typeof globalThis.fetch,
  origin: string,
): Promise<LordsState> {
  const controller = new AbortController();
  const cancel = () => controller.abort();
  signal.addEventListener("abort", cancel, { once: true });
  if (signal.aborted) cancel();
  let timedOut = false;
  const timer = setTimeout(() => {
    timedOut = true;
    controller.abort();
  }, 10_000);
  try {
    const response = await fetch(`${origin}/lords/${id}`, {
      signal: controller.signal,
    });
    if (response.status === 404) return { kind: "not-found" };
    if (!response.ok)
      throw new Error(
        "The registered interests service is unavailable. Try again in a moment.",
      );
    const body: unknown = await response.json();
    const details = parseLordsDetails(body);
    if (details.member.id !== id) return malformed();
    return { kind: "lord", details };
  } catch (error) {
    if (signal.aborted) throw error;
    return {
      kind: "error",
      message: timedOut
        ? "Registered interests are taking too long. Try again."
        : error instanceof Error &&
            !(error instanceof TypeError || error instanceof SyntaxError)
          ? error.message
          : "Couldn’t connect to registered interests. Check your connection and try again.",
    };
  } finally {
    clearTimeout(timer);
    signal.removeEventListener("abort", cancel);
  }
}
