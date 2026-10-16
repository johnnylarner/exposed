import {
  searchConfiguration,
  type Entity,
  type SearchMatchSource,
} from "../search";
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
      typeof value.id !== "string" ||
      (value.kind !== "MP" && value.kind !== "Funder") ||
      (value.funder_kind !== null && typeof value.funder_kind !== "string")
    ) {
      throw new Error("Search returned an unexpected response. Try again.");
    }
    const source = value.match_source;
    if (!isRecord(source))
      throw new Error("Search returned an unexpected match source. Try again.");
    let matchSource: SearchMatchSource;
    switch (source.kind) {
      case "name":
        matchSource = { kind: "name" };
        break;
      case "alias":
        if (typeof source.name !== "string" || !source.name.trim())
          throw new Error(
            "Search returned an unexpected match source. Try again.",
          );
        matchSource = { kind: "alias", name: source.name };
        break;
      default:
        throw new Error(
          "Search returned an unexpected match source. Try again.",
        );
    }
    if (value.kind === "MP") {
      if (
        !/^[1-9]\d*$/u.test(value.id) ||
        Number(value.id) > 2147483647 ||
        value.funder_kind !== null ||
        matchSource.kind !== "name"
      )
        throw new Error(
          "Search returned an unexpected MP identity. Try again.",
        );
      return {
        id: value.id,
        name: value.name,
        kind: "MP",
        funderKind: null,
        matchSource,
      };
    }
    if (
      !/^[\da-f]{8}-[\da-f]{4}-[\da-f]{4}-[\da-f]{4}-[\da-f]{12}$/iu.test(
        value.id,
      )
    )
      throw new Error(
        "Search returned an unexpected funder identity. Try again.",
      );
    return {
      id: value.id,
      name: value.name,
      kind: "Funder",
      funderKind: value.funder_kind,
      matchSource,
    };
  });
}

export async function searchEntities(
  term: string,
  signal: AbortSignal,
  strictness: number,
  fetch: typeof globalThis.fetch,
  origin: string,
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
    const response = await fetch(`${origin}/search?${params}`, {
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
