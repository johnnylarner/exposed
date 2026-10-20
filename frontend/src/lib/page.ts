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
      route: Exclude<DetailRoute, { kind: "search" }>;
      result: Exclude<DetailState, { kind: "loading" }>;
    };

export const pagePresentation = {
  landing: {
    target: "landing",
    label: "content",
    title: "Exposed",
    description:
      "Explore UK MPs' declared financial interests and the people and organisations behind them.",
  },
  methodology: {
    target: "methodology",
    label: "methodology",
    title: "Methodology · Exposed",
    description: "Sources and methodology for Exposed.",
  },
  search: {
    target: "search",
    label: "search",
    title: "Search MPs and funders · Exposed",
    description:
      "Find MPs and funders in the Register of Members' Financial Interests.",
  },
  detail: {
    target: "details",
    label: "details",
    title: "Financial interests · Exposed",
    description:
      "Explore declared financial interests in the Register of Members' Financial Interests.",
  },
} satisfies Record<
  PageModel["kind"],
  { target: string; label: string; title: string; description: string }
>;
