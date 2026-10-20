// Search defaults live here. The API applies
// max_entries independently to MPs and funders, then merges by similarity.
export const searchConfiguration = {
  minimumCharacters: 3,
  debounceMs: 250,
  maxEntriesPerType: 10,
  strictness: 0.8,
  timeoutMs: 10_000,
};

export type SearchMatchSource =
  { kind: "name" } | { kind: "alias"; name: string };

export type Entity =
  | {
      id: string;
      name: string;
      kind: "MP";
      funderKind: null;
      matchSource: { kind: "name" };
    }
  | {
      id: string;
      name: string;
      kind: "Funder";
      funderKind: string | null;
      matchSource: SearchMatchSource;
    };

export function entityHref(
  entity: Pick<Entity, "kind" | "id">,
  params: Pick<URLSearchParams, "get">,
): string {
  const retained = new URLSearchParams();
  const value = params.get("q");
  if (value !== null) retained.set("q", value);
  const query = retained.size ? `?${retained}` : "";
  return `/${entity.kind === "MP" ? "members" : "funders"}/${encodeURIComponent(entity.id)}${query}`;
}

export function entityType(entity: Entity): string {
  if (entity.kind === "MP") return "MP";
  return `Funder: ${funderKindLabel(entity.funderKind)}`;
}

export function funderKindLabel(value: string | null): string {
  const kind = value?.trim();
  if (!kind || kind.toLowerCase() === "not specified") return "Unclassified";
  if (kind.toLowerCase() === "individual") return "Individual";
  if (kind.toLowerCase() === "company") return "Company";
  if (kind.toLowerCase() === "trade union") return "Trade union";
  return kind;
}

export function funderKindIcon(value: string | null) {
  switch (funderKindLabel(value).toLowerCase()) {
    case "individual":
      return "person";
    case "company":
      return "building";
    case "trade union":
      return "group";
    case "unclassified":
      return "question";
    default:
      return "tag";
  }
}

// Plain-text segments keep source names intact and never interpret names or
// user input as markup. Matching here only highlights; it never changes rank.
export function highlightName(name: string, term: string) {
  const tokens = [...new Set(term.trim().split(/\s+/u).filter(Boolean))]
    .sort((a, b) => b.length - a.length)
    .map((token) => token.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"));
  if (!tokens.length) return [{ text: name, matched: false }];
  const pattern = new RegExp(tokens.join("|"), "giu");
  const segments: { text: string; matched: boolean }[] = [];
  let position = 0;
  for (const match of name.matchAll(pattern)) {
    const start = match.index;
    if (start > position)
      segments.push({ text: name.slice(position, start), matched: false });
    segments.push({ text: match[0], matched: true });
    position = start + match[0].length;
  }
  if (position < name.length)
    segments.push({ text: name.slice(position), matched: false });
  return segments;
}
