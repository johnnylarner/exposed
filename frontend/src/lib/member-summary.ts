import { formatAmount, type MemberDetails } from "./entity-details";

export interface EarningsWeek {
  start: string;
  amount: string | null;
  incomplete: boolean;
}

function isEarnings(name: string): boolean {
  const label = name.trim().replace(/\s+/gu, " ").toLowerCase();
  return (
    label === "employment" ||
    label === "employment and earnings" ||
    label === "employment and earnings - ad hoc payments" ||
    label === "employment and earnings - ongoing paid employment"
  );
}

function sum(values: string[]): string | null {
  if (!values.length) return null;
  const scale = Math.max(
    ...values.map((value) => value.split(".")[1]?.length ?? 0),
  );
  const total = values.reduce((total, value) => {
    const [whole, fraction = ""] = value.replace(/^-/, "").split(".");
    const coefficient = BigInt(whole + fraction.padEnd(scale, "0"));
    return total + (value.startsWith("-") ? -coefficient : coefficient);
  }, 0n);
  const digits = (total < 0n ? -total : total)
    .toString()
    .padStart(scale + 1, "0");
  return `${total < 0n ? "-" : ""}${scale ? `${digits.slice(0, -scale)}.${digits.slice(-scale)}` : digits}`;
}

export function formatPounds(amount: string | null): string {
  if (amount === null) return formatAmount(null);
  const unsigned = amount.replace(/^-/, "");
  const [whole, fraction = ""] = unsigned.split(".");
  return `${amount.startsWith("-") ? "−" : ""}£${formatAmount(`${whole}.${fraction.padEnd(2, "0")}`)}`;
}

export function summarizeMember(details: MemberDetails) {
  const pounds = details.summary.currencies.find(
    (group) => group.currency === "GBP",
  );
  const buckets = new Map<string, { amounts: string[]; incomplete: boolean }>();
  let undated = false;
  for (const declaration of details.declarations) {
    if (!isEarnings(declaration.categoryName)) continue;
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
    date.setUTCDate(date.getUTCDate() - ((date.getUTCDay() + 6) % 7));
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
  const earnings: EarningsWeek[] = [];
  if (first !== undefined && last !== undefined) {
    const date = new Date(`${first}T00:00:00Z`);
    for (
      let start = first;
      start <= last;
      start = date.toISOString().slice(0, 10)
    ) {
      const bucket = buckets.get(start);
      earnings.push({
        start,
        amount: bucket ? sum(bucket.amounts) : "0",
        incomplete: bucket?.incomplete ?? false,
      });
      date.setUTCDate(date.getUTCDate() + 7);
    }
  }
  return {
    total: pounds?.amount ?? null,
    categories: pounds?.categories ?? [],
    earnings,
    undated,
  };
}
