import {
  memberIdPattern,
  funderIdPattern,
  type Total,
} from "../entity-details";
import type {
  DetailRoute,
  DetailState,
  MemberProfile,
  MemberDetails,
  FunderDetails,
  MemberDeclaration,
  FunderDeclaration,
  FundingEntry,
} from "../entity-details";
function malformed(): never {
  throw new Error("Details returned an unexpected response. Try again.");
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
function count(value: unknown): number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0
    ? value
    : malformed();
}
function list(value: unknown): unknown[] {
  return Array.isArray(value) ? value : malformed();
}
function decimal(value: unknown): string | null {
  const result = nullableText(value);
  if (result !== null && !/^-?\d+(?:\.\d+)?$/u.test(result)) return malformed();
  return result;
}
function identity(value: unknown, kind: "MP" | "Funder"): string {
  const id = text(value);
  if (
    kind === "MP"
      ? !memberIdPattern.test(id) || Number(id) > 2147483647
      : !funderIdPattern.test(id)
  )
    return malformed();
  return id;
}
function memberProfile(value: unknown): MemberProfile {
  const row = record(value);
  if (typeof row.is_current_commons !== "boolean") return malformed();
  return {
    id: identity(row.id, "MP"),
    name: text(row.name),
    partyName: text(row.party_name),
    membershipFrom: text(row.membership_from),
    isCurrentCommons: row.is_current_commons,
  };
}
function integer(value: unknown): number {
  return typeof value === "number" && Number.isSafeInteger(value)
    ? value
    : malformed();
}
function timestamp(value: unknown): string | null {
  const result = nullableText(value);
  if (result !== null && !Number.isFinite(Date.parse(result)))
    return malformed();
  return result;
}
function declaration(value: unknown): MemberDeclaration {
  const row = record(value);
  return {
    sourceId: count(row.source_id),
    categoryId: integer(row.category_id),
    categoryName: text(row.category_name),
    registeredAt: timestamp(row.registered_at),
    entries: list(row.entries).map((value) => {
      const entry = record(value);
      const funder = entry.funder === null ? null : record(entry.funder);
      return {
        funder:
          funder === null
            ? null
            : { id: identity(funder.id, "Funder"), name: text(funder.name) },
        amount: decimal(entry.amount),
        currency: nullableText(entry.currency),
        paymentType: nullableText(entry.payment_type),
      };
    }),
  };
}
function declaredPeriod(value: unknown) {
  const row = record(value);
  if (typeof row.incomplete !== "boolean") return malformed();
  return {
    amount: decimal(row.amount),
    declarationCount: count(row.declaration_count),
    incomplete: row.incomplete,
  };
}
function declaredTotals(value: unknown) {
  const row = record(value);
  const start = timestamp(row.current_term_start);
  if (start === null) return malformed();
  return {
    currentTermStart: start,
    currentTerm: declaredPeriod(row.current_term),
    historical: declaredPeriod(row.historical),
  };
}
function readMember(value: unknown): MemberDetails {
  const row = record(value);
  const summary = record(row.summary);
  return {
    member: memberProfile(row.member),
    declarations: list(row.declarations).map(declaration),
    summary: {
      declaredTotals: declaredTotals(summary.declared_totals),
      entryCount: count(summary.entry_count),
      unknownCurrencyCount: count(summary.unknown_currency_count),
      firstRegisteredAt: timestamp(summary.first_registered_at),
      lastRegisteredAt: timestamp(summary.last_registered_at),
      undatedDeclarationCount: count(summary.undated_declaration_count),
      currencies: list(summary.currencies).map((value) => {
        const group = record(value);
        const currency = text(group.currency);
        if (!currency.trim()) return malformed();
        return {
          ...total(group),
          currency,
          categories: list(group.categories).map((value) => {
            const category = record(value);
            return {
              ...total(category),
              categoryId: integer(category.category_id),
              categoryName: text(category.category_name),
            };
          }),
        };
      }),
    },
  };
}
function total(row: Record<string, unknown>): Total {
  return {
    amount: decimal(row.amount),
    entryCount: count(row.entry_count),
    unknownAmountCount: count(row.unknown_amount_count),
  };
}
function readFunder(value: unknown): FunderDetails {
  const row = record(value);
  const funder = record(row.funder);
  return {
    declarations: list(row.declarations).map((value) => {
      const source = record(value);
      return {
        member: memberProfile(source.member),
        declaration: declaration(source.declaration),
      };
    }),
    funder: {
      id: identity(funder.id, "Funder"),
      name: text(funder.name),
      funderKind: nullableText(funder.funder_kind),
      companyNumber: nullableText(funder.company_number),
      aliases: list(funder.aliases).map(text),
    },
    entryCount: count(row.entry_count),
    unknownCurrencyCount: count(row.unknown_currency_count),
    recipientLimit: count(row.recipient_limit),
    currencies: list(row.currencies).map((value) => {
      const group = record(value);
      const currency = text(group.currency);
      if (!currency.trim()) return malformed();
      return {
        currency,
        amount: decimal(group.amount),
        parties: list(group.parties).map((value) => {
          const party = record(value);
          return { ...total(party), partyName: text(party.party_name) };
        }),
        topRecipients: list(group.top_recipients).map((value) => {
          const recipient = record(value);
          return {
            ...total(recipient),
            member: memberProfile(recipient.member),
          };
        }),
      };
    }),
  };
}

export async function loadDetails(
  route: Extract<DetailRoute, { kind: "member" | "funder" }>,
  signal: AbortSignal,
  fetch: typeof globalThis.fetch,
  origin: string,
): Promise<Exclude<DetailState, { kind: "loading" }>> {
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
    const response = await fetch(
      `${origin}/${route.kind === "member" ? "members" : "funders"}/${route.id}`,
      { signal: controller.signal },
    );
    if (response.status === 404) return { kind: "not-found" };
    if (!response.ok)
      throw new Error(
        "The details service is unavailable. Try again in a moment.",
      );
    const body: unknown = await response.json();
    const state: DetailState =
      route.kind === "member"
        ? { kind: "member", details: readMember(body) }
        : { kind: "funder", details: readFunder(body) };
    if (
      (state.kind === "member"
        ? state.details.member.id
        : state.details.funder.id
      ).toLowerCase() !== route.id.toLowerCase()
    )
      return malformed();
    return state;
  } catch (error) {
    if (signal.aborted) throw error;
    return {
      kind: "error",
      message: timedOut
        ? "Details are taking too long. Try again."
        : error instanceof Error &&
            !(error instanceof TypeError || error instanceof SyntaxError)
          ? error.message
          : "Couldn’t connect to details. Check your connection and try again.",
    };
  } finally {
    clearTimeout(timer);
    signal.removeEventListener("abort", cancel);
  }
}
