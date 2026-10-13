import type { FunderDetails } from "./entity-details";

const colors = [
  "#df8550",
  "#6587ad",
  "#b39e53",
  "#809c79",
  "#9a83b5",
  "#bf747c",
];

type Party = FunderDetails["currencies"][number]["parties"][number];

function scaledAmount(amount: string | null, scale: number): bigint | null {
  if (amount === null) return null;
  const [whole, fraction = ""] = amount.split(".");
  return BigInt(whole + fraction.padEnd(scale, "0"));
}

function decimalAmount(value: bigint, scale: number): string {
  const digits = (value < 0n ? -value : value)
    .toString()
    .padStart(scale + 1, "0");
  const amount =
    scale === 0
      ? digits
      : `${digits.slice(0, -scale)}.${digits.slice(-scale)}`.replace(
          /\.?0+$/u,
          "",
        );
  return `${value < 0n ? "-" : ""}${amount}`;
}

export function partyChart(
  parties: FunderDetails["currencies"][number]["parties"],
) {
  const scale = Math.max(
    0,
    ...parties.map((party) => party.amount?.split(".")[1]?.length ?? 0),
  );
  const grouped: Party[] = [];
  let labour: { party: Party; amount: bigint | null } | undefined;
  for (const party of parties) {
    if (party.partyName !== "Labour" && party.partyName !== "Labour (Co-op)") {
      grouped.push(party);
      continue;
    }
    const amount = scaledAmount(party.amount, scale);
    if (labour === undefined) {
      labour = { party: { ...party, partyName: "Labour" }, amount };
      grouped.push(labour.party);
    } else {
      if (amount !== null) labour.amount = (labour.amount ?? 0n) + amount;
      labour.party.entryCount += party.entryCount;
      labour.party.unknownAmountCount += party.unknownAmountCount;
    }
  }
  if (labour !== undefined) {
    labour.party.amount =
      labour.amount === null ? null : decimalAmount(labour.amount, scale);
  }
  const values = grouped.map(({ amount }) => {
    const value = scaledAmount(amount, scale) ?? 0n;
    return value > 0n ? value : 0n;
  });
  const total = values.reduce((sum, value) => sum + value, 0n);
  let start = 0;
  return grouped.map((party, index) => {
    const share =
      total === 0n ? 0 : Number((values[index] * 1000000n) / total) / 1000000;
    const angle = share * Math.PI * 2;
    const end = start + angle;
    const path =
      share === 0 || share === 1
        ? ""
        : `M 100 100 L ${100 + 90 * Math.sin(start)} ${100 - 90 * Math.cos(start)} A 90 90 0 ${share > 0.5 ? 1 : 0} 1 ${100 + 90 * Math.sin(end)} ${100 - 90 * Math.cos(end)} Z`;
    start = end;
    return { party, share, path, color: colors[index % colors.length] };
  });
}
