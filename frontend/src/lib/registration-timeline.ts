import { sumAmounts } from "./sum-amounts";
import type { MemberDeclaration } from "./entity-details";

export interface RegistrationMonth {
  start: string;
  amount: string | null;
  incomplete: boolean;
}

export function registrationTimeline(declarations: MemberDeclaration[]) {
  const buckets = new Map<string, { amounts: string[]; incomplete: boolean }>();
  let undated = false;
  for (const declaration of declarations) {
    const entries = declaration.entries.filter(
      (entry) => entry.currency === "GBP" || !entry.currency?.trim(),
    );
    if (!entries.length) continue;
    if (declaration.registeredAt === null) {
      undated = true;
      continue;
    }
    const date = new Date(declaration.registeredAt);
    date.setUTCHours(0, 0, 0, 0);
    date.setUTCDate(1);
    const start = date.toISOString().slice(0, 10);
    const bucket = buckets.get(start) ?? { amounts: [], incomplete: false };
    for (const entry of entries) {
      if (entry.currency === "GBP" && entry.amount !== null)
        bucket.amounts.push(entry.amount);
      else bucket.incomplete = true;
    }
    buckets.set(start, bucket);
  }
  const dates = [...buckets.keys()].sort();
  const first = dates[0];
  const last = dates.at(-1);
  const months: RegistrationMonth[] = [];
  if (first !== undefined && last !== undefined) {
    const date = new Date(`${first}T00:00:00Z`);
    for (
      let start = first;
      start <= last;
      start = date.toISOString().slice(0, 10)
    ) {
      const bucket = buckets.get(start);
      months.push({
        start,
        amount: bucket ? sumAmounts(bucket.amounts) : "0",
        incomplete: bucket?.incomplete ?? false,
      });
      date.setUTCMonth(date.getUTCMonth() + 1);
    }
  }
  return { months, undated };
}
