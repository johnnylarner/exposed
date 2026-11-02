import { error, redirect } from "@sveltejs/kit";
import { rankingCategories, rankingHref } from "../../lib/rankings";
import { readPeriod } from "../../lib/reporting-period";
import type { PageServerLoad } from "./$types";

export const load = (({ url }) => {
  try {
    readPeriod(url.searchParams);
  } catch {
    error(400, "Invalid reporting period");
  }
  const category = url.searchParams.get("category") ?? "members";
  const selected = rankingCategories.find(
    (option) => option.id === category,
  )?.id;
  if (!selected) error(400, "Invalid ranking category");
  redirect(308, rankingHref(selected, url.searchParams));
}) satisfies PageServerLoad;
