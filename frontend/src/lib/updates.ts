import type { CategoryWheelOption } from "./ui";

export interface EntityUpdate {
  id: string;
  name: string;
  declarationCount: number;
  latestRegistration: string;
}
export interface MonthlyUpdates {
  monthStart: string;
  monthEnd: string;
  members: EntityUpdate[];
  companies: EntityUpdate[];
}
export function updateMonth(value: string): string {
  return new Intl.DateTimeFormat("en-GB", {
    month: "long",
    year: "numeric",
    timeZone: "UTC",
  }).format(new Date(value));
}
export function updateDate(value: string): string {
  return new Intl.DateTimeFormat("en-GB", {
    day: "numeric",
    month: "long",
    year: "numeric",
    timeZone: "UTC",
  }).format(new Date(value));
}

export type UpdateCategory = "members" | "companies";
export const updateSortOptions = [
  { value: "newest", label: "Newest first" },
  { value: "oldest", label: "Oldest first" },
] as const;
export type UpdateSort = (typeof updateSortOptions)[number]["value"];
export interface UpdateView {
  category: UpdateCategory;
  sort: UpdateSort;
  visible: number;
}
const names = new Intl.Collator("en-GB", { sensitivity: "base" });
function compareText(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}
function preciseTimestamp(value: string): string {
  const [seconds, fraction = ""] = value.slice(0, -1).split(".");
  return `${seconds}.${fraction.padEnd(9, "0")}Z`;
}
export function sortUpdates(
  rows: readonly EntityUpdate[],
  sort: UpdateSort,
): EntityUpdate[] {
  return [...rows].sort((left, right) => {
    const date = compareText(
      preciseTimestamp(left.latestRegistration),
      preciseTimestamp(right.latestRegistration),
    );
    return (
      date * (sort === "newest" ? -1 : 1) ||
      names.compare(left.name, right.name) ||
      compareText(left.name, right.name) ||
      compareText(left.id, right.id)
    );
  });
}
export const updateCategories = {
  members: {
    id: "members",
    prefix: "members",
    kind: "MP",
    label: "MPs",
    icon: "parliament",
    accent: "member",
    path: "M8 142 A134 134 0 0 1 276 142 L186 142 A44 44 0 0 0 98 142 Z",
    rimPath: "M8 142 A134 134 0 0 1 276 142",
    left: 0,
    top: 0,
    width: 284,
    x: 142,
    y: 58,
  },
  companies: {
    id: "companies",
    prefix: "funders",
    kind: "Company",
    label: "Companies",
    icon: "building",
    accent: "funder",
    path: "M276 0 A134 134 0 0 1 8 0 L98 0 A44 44 0 0 0 186 0 Z",
    rimPath: "M276 0 A134 134 0 0 1 8 0",
    left: 0,
    top: 142,
    width: 284,
    x: 142,
    y: 84,
  },
} satisfies Record<
  UpdateCategory,
  CategoryWheelOption<UpdateCategory> & { prefix: string; kind: string }
>;
