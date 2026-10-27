import { error, redirect } from "@sveltejs/kit";
import { readPeriod } from "../../lib/reporting-period";
import { readDetailRoute } from "../../lib/entity-details";
import { searchConfiguration } from "../../lib/search";
import { loadDetails } from "../../lib/server/entity-details";
import { searchEntities } from "../../lib/server/search";
import type { PageModel } from "../../lib/page";
import type { PageServerLoad } from "./$types";
export const load = (async ({ url, fetch, request, depends }) => {
  if (url.searchParams.has("strictness")) {
    const canonical = new URL(url);
    canonical.searchParams.delete("strictness");
    if (canonical.pathname === "/") canonical.pathname = "/search";
    redirect(308, canonical.pathname + canonical.search);
  }
  depends("exposed:page");
  if (url.pathname === "/") {
    if (url.searchParams.has("q")) redirect(307, `/search${url.search}`);
    return { model: { kind: "landing" } satisfies PageModel };
  }
  if (url.pathname === "/methodology")
    return { model: { kind: "methodology" } satisfies PageModel };
  const route = readDetailRoute(url.pathname);
  const origin = (
    process.env.EXPOSED_API_ORIGIN || "http://127.0.0.1:6999"
  ).replace(/\/$/, "");
  let model: PageModel;
  if (route.kind !== "search") {
    let period;
    try {
      period = readPeriod(url.searchParams);
    } catch {
      error(400, "Invalid reporting period");
    }
    model = {
      kind: "detail",
      period,
      route,
      result:
        route.kind === "invalid"
          ? { kind: "not-found" }
          : await loadDetails(route, request.signal, fetch, origin, period),
    };
  } else {
    const query = url.searchParams.get("q") ?? "";
    const term = query.trim();
    model = { kind: "search", query, result: { status: "idle" } };
    if ([...term].length >= searchConfiguration.minimumCharacters) {
      try {
        model.result = {
          status: "success",
          term,
          entities: await searchEntities(term, request.signal, fetch, origin),
        };
      } catch (error) {
        model.result = {
          status: "error",
          term,
          message:
            error instanceof Error
              ? error.message
              : "Search couldn’t load. Try again.",
        };
      }
    }
  }
  return { model };
}) satisfies PageServerLoad;
