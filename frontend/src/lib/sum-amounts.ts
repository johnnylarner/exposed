export function sumAmounts(values: string[]): string | null {
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
