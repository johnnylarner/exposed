import { registrationTimeline } from "./registration-timeline";
import { formatAmount, type MemberDetails } from "./entity-details";

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
    declaredTotals: details.summary.declaredTotals,
    categories: pounds?.categories ?? [],
    earnings,
    undated,
  };
}
