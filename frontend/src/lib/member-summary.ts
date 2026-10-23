import { sumAmounts } from "./sum-amounts";
import { formatAmount, type MemberDetails } from "./entity-details";

export interface MemberMoneySource {
  funder: MemberDetails["declarations"][number]["entries"][number]["funder"];
  amount: string | null;
  entryCount: number;
  missingGbpAmountCount: number;
  unknownCurrencyCount: number;
  foreignCurrencyCount: number;
}

function compareAmounts(left: string | null, right: string | null): number {
  if (left === null) return right === null ? 0 : 1;
  if (right === null) return -1;
  const difference = sumAmounts([
    right,
    left.startsWith("-") ? left.slice(1) : `-${left}`,
  ]);
  if (difference === null || /^-?0(?:\.0+)?$/u.test(difference)) return 0;
  return difference.startsWith("-") ? -1 : 1;
}

function moneySources(details: MemberDetails): MemberMoneySource[] {
  const groups = new Map<
    string | null,
    { source: MemberMoneySource; amounts: string[] }
  >();
  for (const declaration of details.declarations) {
    for (const entry of declaration.entries) {
      const key = entry.funder?.id ?? null;
      let group = groups.get(key);
      if (!group) {
        group = {
          source: {
            funder: entry.funder,
            amount: null,
            entryCount: 0,
            missingGbpAmountCount: 0,
            unknownCurrencyCount: 0,
            foreignCurrencyCount: 0,
          },
          amounts: [],
        };
        groups.set(key, group);
      }
      group.source.entryCount += 1;
      if (entry.currency === "GBP") {
        if (entry.amount === null) group.source.missingGbpAmountCount += 1;
        else group.amounts.push(entry.amount);
      } else if (!entry.currency?.trim())
        group.source.unknownCurrencyCount += 1;
      else group.source.foreignCurrencyCount += 1;
    }
  }
  return Array.from(groups.values(), ({ source, amounts }) => ({
    ...source,
    amount: sumAmounts(amounts),
  })).sort(
    (left, right) =>
      compareAmounts(left.amount, right.amount) ||
      (left.funder?.name ?? "Source unavailable").localeCompare(
        right.funder?.name ?? "Source unavailable",
        "en",
      ) ||
      (left.funder?.id ?? "").localeCompare(right.funder?.id ?? ""),
  );
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
  return {
    total: pounds?.amount ?? null,
    firstRegisteredAt: details.summary.firstRegisteredAt,
    declaredTotals: details.summary.declaredTotals,
    categories: pounds?.categories ?? [],
    sources: moneySources(details),
  };
}
