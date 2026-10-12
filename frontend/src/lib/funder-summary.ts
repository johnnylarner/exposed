import type { FunderDetails } from "./entity-details";

const colors = [
  "#df8550",
  "#6587ad",
  "#b39e53",
  "#809c79",
  "#9a83b5",
  "#bf747c",
];

export function partyChart(
  parties: FunderDetails["currencies"][number]["parties"],
) {
  const scale = Math.max(
    0,
    ...parties.map((party) => party.amount?.split(".")[1]?.length ?? 0),
  );
  const values = parties.map(({ amount }) => {
    if (amount === null || amount.startsWith("-")) return 0n;
    const [whole, fraction = ""] = amount.split(".");
    return BigInt(whole + fraction.padEnd(scale, "0"));
  });
  const total = values.reduce((sum, value) => sum + value, 0n);
  let start = 0;
  return parties.map((party, index) => {
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
