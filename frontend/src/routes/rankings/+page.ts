import { error } from "@sveltejs/kit";
import { rankingCategories } from "../../lib/rankings";
import type { PageLoad } from "./$types";

export const load = (({ data, url }) => {
  const category = url.searchParams.get("category") ?? "members";
  const selected = rankingCategories.find(
    (option) => option.id === category,
  )?.id;
  if (!selected) error(400, "Invalid ranking category");
  return { ...data, selected };
}) satisfies PageLoad;
