import { registrationTimeline } from "./registration-timeline";
import { sumAmounts } from "./sum-amounts";
import {
  formatAmount,
  type MemberDeclaration,
  type MemberDetails,
} from "./entity-details";

// https://api.parliament.uk/regnal-years/parliament-periods/59
export const CURRENT_PARLIAMENT_START = "2024-07-09T00:00:00Z";

function declaredTotal(declarations: MemberDeclaration[]) {
  const entries = declarations.flatMap((declaration) => declaration.entries);
  return {
    declarationCount: declarations.length,
    amount: sumAmounts(
      entries.flatMap((entry) =>
        entry.currency === "GBP" && entry.amount !== null ? [entry.amount] : [],
      ),
    ),
    incomplete: entries.some(
      (entry) =>
        !entry.currency?.trim() ||
        (entry.currency === "GBP" && entry.amount === null),
    ),
  };
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
  const earningsDeclarations = details.declarations.filter((declaration) =>
    isEarnings(declaration.categoryName),
  );
  const { months: earnings, undated } =
    registrationTimeline(earningsDeclarations);
  return {
    total: pounds?.amount ?? null,
    firstRegisteredAt: details.summary.firstRegisteredAt,
    declaredTotals: {
      currentTerm: declaredTotal(
        details.declarations.filter(
          (declaration) =>
            declaration.registeredAt !== null &&
            Date.parse(declaration.registeredAt) >=
              Date.parse(CURRENT_PARLIAMENT_START),
        ),
      ),
      historical: declaredTotal(details.declarations),
    },
    categories: pounds?.categories ?? [],
    earnings,
    undated,
  };
}
