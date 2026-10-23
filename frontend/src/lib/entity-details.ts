import { searchQueryString } from "./search-query";

export type DetailRoute =
  | { kind: "search" }
  | { kind: "member"; id: string; view: "summary" | "history" }
  | { kind: "funder"; id: string; view: "summary" | "history" }
  | { kind: "invalid" };

export const funderIdPattern =
  /^[\da-f]{8}-[\da-f]{4}-[\da-f]{4}-[\da-f]{4}-[\da-f]{12}$/iu;
export const memberIdPattern = /^[1-9]\d*$/u;

export function readDetailRoute(path: string): DetailRoute {
  if (path === "/search") return { kind: "search" };
  const member = /^\/members\/([^/]+)(?:\/(history))?\/?$/u.exec(path);
  if (
    member &&
    memberIdPattern.test(member[1]) &&
    Number(member[1]) <= 2147483647
  )
    return {
      kind: "member",
      id: member[1],
      view: member[2] ? "history" : "summary",
    };
  const funder = /^\/funders\/([^/]+)(?:\/(history))?\/?$/u.exec(path);
  if (funder && funderIdPattern.test(funder[1]))
    return {
      kind: "funder",
      id: funder[1],
      view: funder[2] ? "history" : "summary",
    };
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

export interface MemberDeclaration {
  sourceId: number;
  categoryId: number;
  categoryName: string;
  registeredAt: string | null;
  entries: FundingEntry[];
}

export interface DeclaredGbpPeriodTotal {
  amount: string | null;
  declarationCount: number;
  incomplete: boolean;
}

export interface DeclaredGbpTotals {
  calendarYear: number;
  currentYear: DeclaredGbpPeriodTotal;
  currentTermStart: string;
  currentTerm: DeclaredGbpPeriodTotal;
  historical: DeclaredGbpPeriodTotal;
}

export interface MemberDetails {
  member: MemberProfile;
  declarations: MemberDeclaration[];
  summary: {
    declaredTotals: DeclaredGbpTotals;
    currencies: (Total & {
      currency: string;
      categories: (Total & { categoryId: number; categoryName: string })[];
    })[];
    entryCount: number;
    unknownCurrencyCount: number;
    firstRegisteredAt: string | null;
    lastRegisteredAt: string | null;
    undatedDeclarationCount: number;
  };
}

export interface Total {
  amount: string | null;
  entryCount: number;
  unknownAmountCount: number;
}

export interface FunderDeclaration {
  member: MemberProfile;
  declaration: MemberDeclaration;
}

export interface FunderDetails {
  declaredTotals: DeclaredGbpTotals;
  firstRegisteredAt: string | null;
  declarations: FunderDeclaration[];
  funder: {
    id: string;
    name: string;
    funderKind: string | null;
    companyNumber: string | null;
    aliases: string[];
  };
  currencies: {
    currency: string;
    amount: string | null;
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

export function backToSearch(params: Pick<URLSearchParams, "get">): string {
  return `/search${searchQueryString(params)}`;
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

export function memberHref(
  id: string,
  view: "summary" | "history",
  params: Pick<URLSearchParams, "get">,
): string {
  const search = searchQueryString(params);
  return `/members/${id}${view === "history" ? "/history" : ""}${search}`;
}

export function categoryBars(
  categories: { amount: string | null }[],
  basis: "total" | "maximum",
): { width: number; negative: boolean }[] {
  const scale = Math.max(
    0,
    ...categories.map(({ amount }) => amount?.split(".")[1]?.length ?? 0),
  );
  const values = categories.map(({ amount }) => {
    if (amount === null) return 0n;
    const [whole, fraction = ""] = amount.replace(/^-/, "").split(".");
    const value = BigInt(whole + fraction.padEnd(scale, "0"));
    return amount.startsWith("-") ? -value : value;
  });
  const magnitudes = values.map((value) => (value < 0n ? -value : value));
  let denominator: bigint;
  let precision: bigint;
  switch (basis) {
    case "total":
      denominator = magnitudes.reduce((total, value) => total + value, 0n);
      precision = 100000000n;
      break;
    case "maximum":
      denominator = magnitudes.reduce(
        (largest, value) => (value > largest ? value : largest),
        0n,
      );
      precision = 100n;
      break;
    default: {
      const exhaustive: never = basis;
      return exhaustive;
    }
  }
  return values.map((value, index) => ({
    negative: value < 0n,
    width:
      denominator === 0n
        ? 0
        : Number((magnitudes[index] * 100n * precision) / denominator) /
          Number(precision),
  }));
}

export function funderHref(
  id: string,
  view: "summary" | "history",
  params: Pick<URLSearchParams, "get">,
): string {
  const search = searchQueryString(params);
  return `/funders/${id}${view === "history" ? "/history" : ""}${search}`;
}
