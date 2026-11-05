import { expect, test } from "./backend";

test("crawler resources describe canonical pages without backend requests", async ({
  request,
  backend,
}) => {
  const backendRequests: string[] = [];
  await backend.route("**/api/*", async (route) => {
    backendRequests.push(route.request().url());
    await route.fulfill({ status: 500 });
  });

  const sitemap = await request.get("/sitemap.xml");
  expect(sitemap.status()).toBe(200);
  expect(sitemap.headers()["content-type"]).toMatch(
    /^(?:application|text)\/xml(?:; charset=utf-8)?$/i,
  );
  const xml = await sitemap.text();
  expect(xml).toMatch(/^<\?xml version="1\.0" encoding="UTF-8"\?>/);
  expect(xml).toContain(
    '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">',
  );
  expect(xml.trimEnd()).toMatch(/<\/urlset>$/);
  const locations = Array.from(
    xml.matchAll(/<loc>(.*?)<\/loc>/g),
    ([, url]) => url,
  );
  expect(locations).toEqual([
    "https://declair.work/",
    "https://declair.work/about",
    "https://declair.work/methodology",
    "https://declair.work/updates",
    "https://declair.work/ranking/mps",
    "https://declair.work/ranking/individuals",
    "https://declair.work/ranking/companies",
    "https://declair.work/ranking/parties",
  ]);
  expect(new Set(locations).size).toBe(8);
  expect(xml).not.toMatch(/<(?:lastmod|priority|changefreq)>/);

  const robots = await request.get("/robots.txt");
  expect(robots.status()).toBe(200);
  expect(robots.headers()["content-type"]).toMatch(
    /^text\/plain(?:; charset=utf-8)?$/i,
  );
  expect(await robots.text()).toBe(
    "User-agent: *\nAllow: /\nSitemap: https://declair.work/sitemap.xml\n",
  );
  expect(backendRequests).toEqual([]);
});
