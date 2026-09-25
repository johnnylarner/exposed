// Search defaults live here. The API applies
// max_entries independently to MPs and funders, then merges by similarity.
export const searchConfiguration = {
  minimumCharacters: 3,
  debounceMs: 250,
  maxEntriesPerType: 10,
  strictness: 1,
  timeoutMs: 10_000,
};

export function readStrictness(value: string | null): number {
  if (!value?.trim()) return searchConfiguration.strictness;
  const parsed = Number(value);
  return Number.isFinite(parsed) && parsed >= 0 && parsed <= 1
    ? Math.round(parsed * 100) / 100
    : searchConfiguration.strictness;
}

export interface Entity {
  name: string;
  kind: "MP" | "Funder";
  funderKind: string | null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function readEntities(body: unknown): Entity[] {
  if (!isRecord(body) || !Array.isArray(body.entities)) {
    throw new Error("Search returned an unexpected response. Try again.");
  }

  return body.entities.map((value: unknown) => {
    if (
      !isRecord(value) ||
      typeof value.name !== "string" ||
      (value.kind !== "MP" && value.kind !== "Funder") ||
      (value.funder_kind !== null && typeof value.funder_kind !== "string")
    ) {
      throw new Error("Search returned an unexpected response. Try again.");
    }
    return {
      name: value.name,
      kind: value.kind,
      funderKind: value.funder_kind,
    };
  });
}

export async function searchEntities(
  term: string,
  signal: AbortSignal,
  strictness: number,
): Promise<Entity[]> {
  const controller = new AbortController();
  let timedOut = false;
  const cancel = () => controller.abort();
  signal.addEventListener("abort", cancel, { once: true });
  if (signal.aborted) cancel();
  const timeout = setTimeout(() => {
    timedOut = true;
    controller.abort();
  }, searchConfiguration.timeoutMs);

  try {
    const params = new URLSearchParams({
      term,
      max_entries: String(searchConfiguration.maxEntriesPerType),
      strictness: String(strictness),
    });
    const response = await fetch(`/api/search?${params}`, {
      signal: controller.signal,
    });
    if (!response.ok) {
      throw new Error(
        "The search service is unavailable. Try again in a moment.",
      );
    }
    const body: unknown = await response.json();
    return readEntities(body);
  } catch (error) {
    if (timedOut) throw new Error("Search is taking too long. Try again.");
    if (signal.aborted) throw error;
    if (error instanceof TypeError || error instanceof SyntaxError) {
      throw new Error(
        "Couldn’t connect to search. Check your connection and try again.",
      );
    }
    throw error;
  } finally {
    clearTimeout(timeout);
    signal.removeEventListener("abort", cancel);
  }
}

export function entityType(entity: Entity): string {
  if (entity.kind === "MP") return "MP";
  const kind = entity.funderKind?.trim();
  if (!kind || kind.toLowerCase() === "not specified") return "Funder";
  if (kind.toLowerCase() === "individual") return "Individual funder";
  if (kind.toLowerCase() === "trade union") return "Trade union";
  return kind;
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
