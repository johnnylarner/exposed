import { error } from "@sveltejs/kit";
import { readPeriod } from "../../lib/reporting-period";
import { rankingCategories } from "../../lib/rankings";
import { loadRankings } from "../../lib/server/rankings";
import type { PageServerLoad } from "./$types";
export const load = (async ({ fetch, request, url }) => {
  let period;
  try {
    period = readPeriod(url.searchParams);
  } catch {
    error(400, "Invalid reporting period");
  }
  const category = url.searchParams.get("category") ?? "members";
  const selected = rankingCategories.find(
    (option) => option.id === category,
  )?.id;
  if (!selected) error(400, "Invalid ranking category");
  const origin = (
    process.env.EXPOSED_API_ORIGIN || "http://127.0.0.1:6999"
  ).replace(/\/$/, "");
  try {
    return {
      period,
      selected,
      rankings: await loadRankings(fetch, request.signal, origin, period),
      error: null,
    };
  } catch {
    return {
      period,
      selected,
      rankings: null,
      error: "Rankings couldn’t load. Try again.",
    };
  }
}) satisfies PageServerLoad;
