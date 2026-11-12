import type { DetailRoute, DetailState } from "./entity-details";
import type { Entity } from "./search";
import type { LordsState } from "./lords";
export type SearchResult =
  | { status: "idle" }
  | { status: "success"; term: string; entities: Entity[] }
  | { status: "error"; term: string; message: string };
export type PageModel =
  | { kind: "landing" }
  | { kind: "methodology" }
  | (Extract<DetailRoute, { kind: "lord" }> & { result: LordsState })
  | { kind: "search"; query: string; result: SearchResult }
  | {
      kind: "detail";
      period: import("./reporting-period").ReportingPeriod;
      route: Exclude<DetailRoute, { kind: "search" | "lord" }>;
      result: Exclude<DetailState, { kind: "loading" }>;
    };

export const pagePresentation = {
  landing: {
    target: "landing",
    label: "content",
    title: "declair",
    description:
      "Explore MPs’ and Lords’ financial interests and the people and organisations behind them.",
  },
  methodology: {
    target: "methodology",
    label: "methodology",
    title: "Methodology · declair",
    description: "Sources and methodology for declair.",
  },
  search: {
    target: "search",
    label: "search",
    title: "Search MPs, Lords and funders · declair",
    description:
      "Find MPs, Lords and funders in Parliament’s registers of interests.",
  },
  lord: {
    target: "lords",
    label: "registered interests",
    title: "Lords registered interests · declair",
    description: "Explore Lords’ registered interests and interest history.",
  },
  detail: {
    target: "details",
    label: "details",
    title: "Financial interests · declair",
    description:
      "Explore declared financial interests in the Register of Members' Financial Interests.",
  },
} satisfies Record<
  PageModel["kind"],
  { target: string; label: string; title: string; description: string }
>;
