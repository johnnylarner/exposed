import { loadUpdates } from "../../lib/server/updates";
import type { PageServerLoad } from "./$types";
export const load = (async ({ fetch, request }) => {
  const origin = (
    process.env.EXPOSED_API_ORIGIN || "http://127.0.0.1:6999"
  ).replace(/\/$/, "");
  try {
    return {
      updates: await loadUpdates(fetch, request.signal, origin),
      error: null,
    };
  } catch {
    return {
      updates: null,
      error: "Latest declarations couldn’t load. Try again.",
    };
  }
}) satisfies PageServerLoad;
