import type { DeclaredGbpPeriodTotal, MemberProfile } from "./entity-details";
import { formatPounds } from "./member-summary";
import type { ChartBar, PieSlice, SignedBar } from "./ui";

export type Quartile = 1 | 2 | 3 | 4;
export type Overview = {
  currentTermStart: string;
  memberCount: number;
  unavailableMemberCount: number;
  incomplete: boolean;
  quartiles: {
    quartile: Quartile;
    memberCount: number;
    minimum: string | null;
    maximum: string | null;
    incompleteMemberCount: number;
  }[];
  topMembers: { member: MemberProfile; total: DeclaredGbpPeriodTotal }[];
  parties: {
    partyId: number;
    partyName: string;
    total: DeclaredGbpPeriodTotal;
    memberCount: number;
  }[];
};
export type OverviewState =
  { kind: "available"; overview: Overview } | { kind: "unavailable" };

function coefficients(amounts: (string | null)[]) {
  const scale = Math.max(
    0,
    ...amounts.map((amount) => amount?.split(".")[1]?.length ?? 0),
  );
  return amounts.map((amount) => {
    if (amount === null) return null;
    const [whole, fraction = ""] = amount.replace(/^-/, "").split(".");
    return (
      BigInt(whole + fraction.padEnd(scale, "0")) *
      (amount.startsWith("-") ? -1n : 1n)
    );
  });
}
function ratio(value: bigint, total: bigint): number {
  return total === 0n ? 0 : Number((value * 1000000n) / total) / 1000000;
}
export function overviewCharts(overview: Overview) {
  const values = coefficients(
    overview.topMembers.map((row) => row.total.amount),
  );
  const positive = values.reduce<bigint>(
    (max, value) => (value !== null && value > max ? value : max),
    0n,
  );
  const negative = values.reduce<bigint>(
    (min, value) => (value !== null && value < min ? value : min),
    0n,
  );
  const extent = positive - negative;
  const members: SignedBar[] = overview.topMembers.map((row, index) => ({
    label: row.member.name,
    value: formatPounds(row.total.amount),
    width:
      ratio(
        values[index] === null
          ? 0n
          : values[index] < 0n
            ? -values[index]
            : values[index],
        extent,
      ) * 100,
    negative: (values[index] ?? 0n) < 0n,
    note: row.total.incomplete ? "Partial" : undefined,
  }));
  const ranges = coefficients(
    overview.quartiles.flatMap((row) => [row.minimum, row.maximum]),
  );
  const low = ranges.reduce<bigint>(
    (min, value) => (value !== null && value < min ? value : min),
    0n,
  );
  const high = ranges.reduce<bigint>(
    (max, value) => (value !== null && value > max ? value : max),
    0n,
  );
  const range = high - low;
  const baseline = 320 - ratio(-low, range) * 280;
  const quartiles: ChartBar[] = overview.quartiles.map((row, index) => {
    const minimum = ranges[index * 2] ?? 0n;
    const maximum = ranges[index * 2 + 1] ?? 0n;
    return {
      x: index * 140 + 30,
      y: 320 - ratio(maximum - low, range) * 280,
      width: 80,
      height: row.memberCount
        ? Math.max(2, ratio(maximum - minimum, range) * 280)
        : 0,
      negative: maximum < 0n,
      title: `Quartile ${row.quartile}: ${row.memberCount} MPs, ${formatPounds(row.minimum)} to ${formatPounds(row.maximum)}`,
      tick: {
        x: index * 140 + 70,
        label: `Q${row.quartile}`,
        anchor: "middle",
      },
    };
  });
  const partyValues = coefficients(
    overview.parties.map((row) => row.total.amount),
  );
  const negativeParty = partyValues.some(
    (value) => value !== null && value < 0n,
  );
  const total = partyValues.reduce<bigint>(
    (sum, value) => sum + (value ?? 0n),
    0n,
  );
  let start = 0;
  const colors = [
    "#b67be5",
    "#64cfb5",
    "#f0c17d",
    "#6587ad",
    "#bf747c",
    "#809c79",
  ];
  const parties: PieSlice[] = overview.parties.map((row, index) => {
    const share = negativeParty ? 0 : ratio(partyValues[index] ?? 0n, total);
    const end = start + share * Math.PI * 2;
    const path = `M 100 100 L ${100 + 90 * Math.sin(start)} ${100 - 90 * Math.cos(start)} A 90 90 0 ${share > 0.5 ? 1 : 0} 1 ${100 + 90 * Math.sin(end)} ${100 - 90 * Math.cos(end)} Z`;
    start = end;
    return {
      share,
      path,
      color: colors[index % colors.length],
      label: row.partyName,
      value: formatPounds(row.total.amount),
      detail: `${formatPounds(row.total.amount)} · ${row.memberCount} MPs${row.total.incomplete ? " · Partial" : ""}`,
    };
  });
  return {
    minimum: formatPounds(
      low === 0n
        ? "0"
        : (overview.quartiles.find((q) => q.minimum !== null)?.minimum ?? null),
    ),
    members,
    memberBaseline: ratio(-negative, extent) * 100,
    quartiles,
    baseline,
    parties,
    negativeParty,
  };
}
