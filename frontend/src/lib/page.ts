import type { DetailRoute, DetailState } from "./entity-details";
import type { Entity } from "./search";
export type SearchResult =
  | { status: "idle" }
  | { status: "success"; term: string; entities: Entity[] }
  | { status: "error"; term: string; message: string };
export type PageModel =
  | { kind: "search"; query: string; strictness: number; result: SearchResult }
  | {
      kind: "detail";
      route: Exclude<DetailRoute, { kind: "search" }>;
      result: Exclude<DetailState, { kind: "loading" }>;
    };
