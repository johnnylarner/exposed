export type DetailRoute =
  | { kind: "search" }
  | { kind: "member"; id: string }
  | { kind: "funder"; id: string }
  | { kind: "invalid" };

const funderIdPattern =
  /^[\da-f]{8}-[\da-f]{4}-[\da-f]{4}-[\da-f]{4}-[\da-f]{12}$/iu;
const memberIdPattern = /^[1-9]\d*$/u;

export function readDetailRoute(path: string): DetailRoute {
  if (path === "/") return { kind: "search" };
  const member = /^\/members\/([^/]+)\/?$/u.exec(path);
  if (
    member &&
    memberIdPattern.test(member[1]) &&
    Number(member[1]) <= 2147483647
  )
    return { kind: "member", id: member[1] };
  const funder = /^\/funders\/([^/]+)\/?$/u.exec(path);
  if (funder && funderIdPattern.test(funder[1]))
    return { kind: "funder", id: funder[1] };
  return { kind: "invalid" };
}

export interface MemberProfile {
  id: string;
  name: string;
  partyName: string;
  membershipFrom: string;
  isCurrentCommons: boolean;
}

export interface FundingEntry {
  funder: { id: string; name: string } | null;
  amount: string | null;
  currency: string | null;
  paymentType: string | null;
}

export interface MemberDetails {
  member: MemberProfile;
  declarations: {
    sourceId: number;
    categoryName: string;
    registeredAt: string | null;
    entries: FundingEntry[];
  }[];
  declarationCount: number;
  declarationLimit: number;
}

interface Total {
  amount: string | null;
  entryCount: number;
  unknownAmountCount: number;
}

export interface FunderDetails {
  funder: {
    id: string;
    name: string;
    funderKind: string | null;
    companyNumber: string | null;
  };
  currencies: {
    currency: string;
    parties: (Total & { partyName: string })[];
    topRecipients: (Total & { member: MemberProfile })[];
  }[];
  entryCount: number;
  unknownCurrencyCount: number;
  recipientLimit: number;
}

export type DetailState =
  | { kind: "loading" }
  | { kind: "member"; details: MemberDetails }
  | { kind: "funder"; details: FunderDetails }
  | { kind: "not-found" }
  | { kind: "error"; message: string };

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
function readMember(value: unknown): MemberDetails {
  const row = record(value);
  return {
    member: memberProfile(row.member),
    declarationCount: count(row.declaration_count),
    declarationLimit: count(row.declaration_limit),
    declarations: list(row.declarations).map((value) => {
      const declaration = record(value);
      const registeredAt = nullableText(declaration.registered_at);
      if (registeredAt !== null && !Number.isFinite(Date.parse(registeredAt)))
        return malformed();
      return {
        sourceId: count(declaration.source_id),
        categoryName: text(declaration.category_name),
        registeredAt,
        entries: list(declaration.entries).map((value) => {
          const entry = record(value);
          const funder = entry.funder === null ? null : record(entry.funder);
          return {
            funder:
              funder === null
                ? null
                : {
                    id: identity(funder.id, "Funder"),
                    name: text(funder.name),
                  },
            amount: decimal(entry.amount),
            currency: nullableText(entry.currency),
            paymentType: nullableText(entry.payment_type),
          };
        }),
      };
    }),
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
    funder: {
      id: identity(funder.id, "Funder"),
      name: text(funder.name),
      funderKind: nullableText(funder.funder_kind),
      companyNumber: nullableText(funder.company_number),
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
): Promise<DetailState> {
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
      `/api/${route.kind === "member" ? "members" : "funders"}/${route.id}`,
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

export function backToSearch(): string {
  const params = new URLSearchParams(window.location.search);
  const retained = new URLSearchParams();
  for (const key of ["q", "strictness"]) {
    const value = params.get(key);
    if (value !== null) retained.set(key, value);
  }
  return retained.size ? `/?${retained}` : "/";
}

export function formatAmount(value: string | null): string {
  if (value === null) return "Amount unavailable";
  const negative = value.startsWith("-");
  const unsigned = negative ? value.slice(1) : value;
  const [whole, fraction] = unsigned.split(".");
  return `${negative ? "−" : ""}${whole.replace(/\B(?=(\d{3})+(?!\d))/gu, ",")}${fraction === undefined ? "" : `.${fraction}`}`;
}

export function registrationDate(value: string): string {
  return new Intl.DateTimeFormat("en-GB", {
    day: "numeric",
    month: "short",
    year: "numeric",
    timeZone: "UTC",
  }).format(new Date(value));
}
