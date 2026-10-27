import type {
  DeclaredGbpTotals,
  DeclaredGbpPeriodTotal,
} from "./entity-details";

export const reportingPeriods = {
  historical: "Historical",
  "current-term": "This parliamentary term",
  "current-year": "This year",
} as const;
export type ReportingPeriod = keyof typeof reportingPeriods;
export function isReportingPeriod(value: unknown): value is ReportingPeriod {
  return typeof value === "string" && Object.hasOwn(reportingPeriods, value);
}
export function readPeriod(
  params: Pick<URLSearchParams, "get">,
): ReportingPeriod {
  const value = params.get("period");
  if (value === null) return "current-term";
  if (isReportingPeriod(value)) return value;
  throw new Error("Invalid reporting period");
}
export function declaredTotal(
  totals: DeclaredGbpTotals,
  period: ReportingPeriod,
): DeclaredGbpPeriodTotal {
  switch (period) {
    case "historical":
      return totals.historical;
    case "current-term":
      return totals.currentTerm;
    case "current-year":
      return totals.currentYear;
    default: {
      const exhaustive: never = period;
      return exhaustive;
    }
  }
}
export function withPeriod(
  destination: string,
  period: ReportingPeriod,
  params?: Pick<URLSearchParams, "get">,
): string {
  const url = new URL(destination, "http://exposed.local");
  url.searchParams.set("period", period);
  const q = params?.get("q");
  if (q !== null && q !== undefined) url.searchParams.set("q", q);
  return url.pathname + url.search;
}
export const periodOptions = Object.entries(reportingPeriods).map(
  ([value, label]) => {
    if (!isReportingPeriod(value)) throw new Error("Invalid period option");
    return { value, label };
  },
);
