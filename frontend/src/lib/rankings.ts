import { withPeriod, type ReportingPeriod } from "./reporting-period";
import { categoryBars } from "./entity-details";
import { formatPounds } from "./member-summary";
import type { DeclaredGbpPeriodTotal } from "./entity-details";
import type { CategoryWheelOption, RankingRow } from "./ui";

export interface RankingEntry {
  id: string;
  name: string;
  context: string | null;
  total: DeclaredGbpPeriodTotal;
}
export type PartyRankingEntry = Omit<RankingEntry, "id" | "name"> & {
  id: string | null;
  name: string | null;
};
/** Complete ordered boards from one reporting-period snapshot. */
export interface Rankings {
  period: ReportingPeriod;
  currentTermStart: string;
  members: RankingEntry[];
  individuals: RankingEntry[];
  companies: RankingEntry[];
  parties: PartyRankingEntry[];
}
export const rankingCategories = [
  {
    id: "members",
    slug: "mps",
    label: "MPs",
    icon: "parliament",
    accent: "member",
    path: "M138 8 A130 130 0 0 0 8 138 L98 138 A40 40 0 0 1 138 98 Z",
    rimPath: "M138 8 A130 130 0 0 0 8 138",
    left: 0,
    top: 0,
    x: 73,
    y: 73,
  },
  {
    id: "individuals",
    slug: "individuals",
    label: "Individuals",
    icon: "person",
    accent: "funder",
    path: "M4 8 A130 130 0 0 1 134 138 L44 138 A40 40 0 0 0 4 98 Z",
    rimPath: "M4 8 A130 130 0 0 1 134 138",
    left: 142,
    top: 0,
    x: 69,
    y: 73,
  },
  {
    id: "companies",
    slug: "companies",
    label: "Companies",
    icon: "building",
    accent: "funder",
    path: "M4 134 A130 130 0 0 0 134 4 L44 4 A40 40 0 0 1 4 44 Z",
    rimPath: "M4 134 A130 130 0 0 0 134 4",
    left: 142,
    top: 142,
    x: 54,
    y: 55,
  },
  {
    id: "parties",
    slug: "parties",
    label: "Parties",
    icon: "group",
    accent: "neutral",
    path: "M8 4 A130 130 0 0 0 138 134 L138 44 A40 40 0 0 1 98 4 Z",
    rimPath: "M8 4 A130 130 0 0 0 138 134",
    left: 0,
    top: 142,
    x: 88,
    y: 55,
  },
] as const satisfies readonly (CategoryWheelOption<string> & {
  slug: string;
})[];
export type RankingCategory = (typeof rankingCategories)[number]["id"];

export function rankingHref(
  category: RankingCategory,
  searchParams?: Pick<URLSearchParams, "toString">,
): string {
  const option = rankingCategories.find((option) => option.id === category)!;
  const params = new URLSearchParams(searchParams?.toString());
  params.delete("category");
  const search = params.toString();
  return `/ranking/${option.slug}${search ? `?${search}` : ""}`;
}

export function rankingRows(
  rankings: Rankings,
  category: RankingCategory,
): RankingRow[] {
  const entries = rankings[category];
  const bars = categoryBars(
    entries.map(({ total }) => ({ amount: total.amount })),
    "maximum",
  );
  return entries.map((entry, index) => ({
    key: `${category}:${entry.id ?? "unknown"}`,
    name: entry.name ?? "Party unavailable",
    context:
      entry.context ??
      `${entry.total.declarationCount} declaration${entry.total.declarationCount === 1 ? "" : "s"}`,
    href:
      category === "parties"
        ? undefined
        : withPeriod(
            `/${category === "members" ? "members" : "funders"}/${entry.id}`,
            rankings.period,
          ),
    amount: formatPounds(entry.total.amount),
    incomplete: entry.total.incomplete,
    width: bars[index].width,
    negative: bars[index].negative,
    accent:
      category === "members"
        ? "member"
        : category === "parties"
          ? "neutral"
          : "funder",
  }));
}
