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
export type UpdateSort = "newest" | "oldest";
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
    path: "M142 8 A134 134 0 0 0 142 276 L142 186 A44 44 0 0 1 142 98 Z",
    rimPath: "M142 8 A134 134 0 0 0 142 276",
    left: 0,
    top: 0,
    x: 52,
    y: 142,
    height: 284,
  },
  companies: {
    id: "companies",
    prefix: "funders",
    kind: "Company",
    label: "Companies",
    icon: "building",
    accent: "funder",
    path: "M0 8 A134 134 0 0 1 0 276 L0 186 A44 44 0 0 0 0 98 Z",
    rimPath: "M0 8 A134 134 0 0 1 0 276",
    left: 142,
    top: 0,
    x: 84,
    y: 142,
    height: 284,
  },
} satisfies Record<
  UpdateCategory,
  CategoryWheelOption<UpdateCategory> & { prefix: string; kind: string }
>;
