import type { DetailRoute, DetailState } from "./entity-details";
import type { Entity } from "./search";
export type SearchResult =
  | { status: "idle" }
  | { status: "success"; term: string; entities: Entity[] }
  | { status: "error"; term: string; message: string };
export type PageModel =
  | { kind: "landing" }
  | { kind: "methodology" }
  | { kind: "search"; query: string; result: SearchResult }
  | {
      kind: "detail";
      period: import("./reporting-period").ReportingPeriod;
      route: Exclude<DetailRoute, { kind: "search" }>;
      result: Exclude<DetailState, { kind: "loading" }>;
    };

export const pagePresentation = {
  landing: {
    target: "landing",
    label: "content",
    title: "declair",
    description:
      "Explore MPs' declared financial interests and the people and organisations behind them.",
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
    title: "Search MPs and funders · declair",
    description:
      "Find MPs and funders in the Register of Members' Financial Interests.",
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
