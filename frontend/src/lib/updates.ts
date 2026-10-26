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
export const updateCategories = {
  members: {
    id: "members",
    prefix: "members",
    kind: "MP",
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
  companies: {
    id: "companies",
    prefix: "funders",
    kind: "Company",
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
} satisfies Record<
  UpdateCategory,
  CategoryWheelOption<UpdateCategory> & { prefix: string; kind: string }
>;
