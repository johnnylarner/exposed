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
