import { error } from "@sveltejs/kit";
import { rankingCategories } from "../../../lib/rankings";
import type { PageLoad } from "./$types";

export const load = (({ data, params }) => {
  const selected = rankingCategories.find(
    (option) => option.slug === params.entity,
  )?.id;
  if (!selected) error(404, "Invalid ranking entity");
  return { ...data, selected };
}) satisfies PageLoad;
