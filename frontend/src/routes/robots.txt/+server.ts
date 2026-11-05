import type { RequestHandler } from "./$types";
import { canonicalOrigin } from "../../lib/server/site";

export const prerender = true;

export const GET: RequestHandler = () =>
  new Response(
    `User-agent: *\nAllow: /\nSitemap: ${canonicalOrigin}/sitemap.xml\n`,
    { headers: { "content-type": "text/plain; charset=utf-8" } },
  );
