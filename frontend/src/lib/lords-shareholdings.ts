import type { RegisteredInterest } from "./lords";

export type ShareholdingSummary =
  | { kind: "labels"; text: string }
  | { kind: "source-preview"; text: string }
  | { kind: "missing"; text: "Interest text unavailable" };

const legalSuffix =
  /^(?:Limited|Ltd\.?|plc|Inc\.?|Corporation|Corp\.?|LLC|LLP|LP|S\.A\.|N\.V\.|SA|AG|NV|SE|GmbH)(?=$|[\s,;.])/iu;
const nameWord =
  /^(?:[\p{Lu}\p{N}][\p{L}\p{N}’'&.+:/%-]*|[\p{Ll}][\p{L}\p{N}]*\p{Lu}[\p{L}\p{N}]*|and|of|the|for|in|de|&|-)$/u;
const ownershipPrefix =
  /^(?:\d+(?:\.\d+)? per cent holding in |\d+(?:\.\d+)? per cent ownership with partner of |Sole shareholder, |Shareholding in )/u;

function parenthesisEnd(source: string, start: number): number | null {
  let depth = 0;
  for (let index = start; index < source.length; index += 1) {
    if (source[index] === "(") depth += 1;
    if (source[index] === ")") {
      depth -= 1;
      if (depth === 0) return index + 1;
    }
  }
  return null;
}

function supportedName(value: string): boolean {
  const words = value
    .replace(/,\s*$/u, "")
    .replace(/[()]/gu, " ")
    .trim()
    .split(/\s+/u);
  return words.length > 0 && words.every((word) => nameWord.test(word));
}

function holdingNames(source: string): string[] | null {
  if (source.length > 16000) return null;
  const names: string[] = [];
  let cursor = 0;
  while (cursor < source.length) {
    const start = cursor;
    let nameEnd: number | null = null;
    for (; cursor < source.length; cursor += 1) {
      const character = source[cursor];
      if (character === "(") {
        const end = parenthesisEnd(source, cursor);
        if (end === null) return null;
        cursor = end - 1;
        continue;
      }
      if (character === ")" || character === "\n" || character === ";")
        return null;
      if (cursor === start || !/[\s,]/u.test(source[cursor - 1])) continue;
      const suffix = source.slice(cursor).match(legalSuffix);
      if (!suffix || !supportedName(source.slice(start, cursor))) continue;
      nameEnd = cursor + suffix[0].length;
      break;
    }
    if (nameEnd === null) return null;
    names.push(source.slice(start, nameEnd).trim());
    cursor = nameEnd;
    while (cursor < source.length) {
      const spacing = source.slice(cursor).match(/^[\t ]*/u)?.[0] ?? "";
      cursor += spacing.length;
      if (source[cursor] !== "(") break;
      const end = parenthesisEnd(source, cursor);
      if (end === null) return null;
      cursor = end;
    }
    const remainder = source.slice(cursor);
    if (/^\.?\s*$/u.test(remainder)) return names;
    const separator = remainder.match(
      /^(?:,\s*(?:and\s+)?|;\s*|\r?\n\s*|and\s+)/u,
    );
    if (!separator) return null;
    cursor += separator[0].length;
    if (cursor === source.length) return null;
  }
  return null;
}

export function shareholdingSummary(
  text: RegisteredInterest["text"],
): ShareholdingSummary {
  const source = text?.trim();
  if (!source) return { kind: "missing", text: "Interest text unavailable" };
  const labels = holdingNames(source.replace(ownershipPrefix, ""));
  if (labels) return { kind: "labels", text: labels.join("; ") };
  const preview = source.replace(/\s+/gu, " ");
  if (preview.length <= 112) return { kind: "source-preview", text: preview };
  const shortened = preview
    .slice(0, 112)
    .replace(/\s+\S*$/u, "")
    .trimEnd();
  return { kind: "source-preview", text: `${shortened}…` };
}
