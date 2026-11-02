import { error } from "@sveltejs/kit";
import { readPeriod } from "../../../lib/reporting-period";
import { loadRankings } from "../../../lib/server/rankings";
import type { PageServerLoad } from "./$types";
export const load = (async ({ fetch, request, url }) => {
  let period;
  try {
    period = readPeriod(url.searchParams);
  } catch {
    error(400, "Invalid reporting period");
  }
  const origin = (
    process.env.EXPOSED_API_ORIGIN || "http://127.0.0.1:6999"
  ).replace(/\/$/, "");
  try {
    return {
      period,
      rankings: await loadRankings(fetch, request.signal, origin, period),
      error: null,
    };
  } catch {
    return {
      period,
      rankings: null,
      error: "Rankings couldn’t load. Try again.",
    };
  }
}) satisfies PageServerLoad;
