import type { RequestHandler } from "./$types";
import { rankingCategories, rankingHref } from "../../lib/rankings";
import { canonicalOrigin } from "../../lib/server/site";

export const prerender = true;

const paths: readonly string[] = [
  "/",
  "/about",
  "/methodology",
  "/updates",
  ...rankingCategories.map(({ id }) => rankingHref(id)),
];

export const GET: RequestHandler = () => {
  const urls = paths.map((path) => {
    const location = `${canonicalOrigin}${path}`
      .replaceAll("&", "&amp;")
      .replaceAll("<", "&lt;")
      .replaceAll(">", "&gt;")
      .replaceAll('"', "&quot;")
      .replaceAll("'", "&apos;");
    return `  <url><loc>${location}</loc></url>`;
  });
  return new Response(
    [
      '<?xml version="1.0" encoding="UTF-8"?>',
      '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">',
      ...urls,
      "</urlset>",
      "",
    ].join("\n"),
    { headers: { "content-type": "application/xml; charset=utf-8" } },
  );
};
