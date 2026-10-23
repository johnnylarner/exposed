import { loadRankings } from "../../lib/server/rankings";
import type { PageServerLoad } from "./$types";
export const load = (async ({ fetch, request }) => {
  const origin = (
    process.env.EXPOSED_API_ORIGIN || "http://127.0.0.1:6999"
  ).replace(/\/$/, "");
  try {
    return {
      rankings: await loadRankings(fetch, request.signal, origin),
      error: null,
    };
  } catch {
    return { rankings: null, error: "Rankings couldn’t load. Try again." };
  }
}) satisfies PageServerLoad;
