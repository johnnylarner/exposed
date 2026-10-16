#!/usr/bin/env node
import assert from "node:assert/strict";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { chromium, expect } from "@playwright/test";

const [baseURL, evidenceDirectory, responseFile] = process.argv.slice(2);
assert(
  baseURL && evidenceDirectory && responseFile,
  "Usage: search.mjs <frontend-loopback-url> <new-evidence-directory> <reference-api-json>",
);
const origin = new URL(baseURL);
assert(
  origin.protocol === "http:" &&
    ["127.0.0.1", "localhost"].includes(origin.hostname) &&
    !origin.username &&
    !origin.password &&
    origin.href === `${origin.origin}/`,
  "Use the owned frontend's HTTP loopback origin without a path or credentials",
);
const evidence = resolve(evidenceDirectory);
await mkdir(evidence);
const browser = await chromium.launch();
try {
  const context = await browser.newContext({ baseURL: origin.origin });
  await context.tracing.start({
    screenshots: true,
    snapshots: true,
    sources: true,
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15_000);
  const requests = [];
  const pageErrors = [];
  const backendRequests = [];
  page.on("request", (request) => {
    const url = new URL(request.url());
    if (url.pathname.startsWith("/api/") || url.origin !== origin.origin)
      backendRequests.push(request.url());
    if (request.resourceType() === "fetch" && url.searchParams.has("q"))
      requests.push(request.url());
  });
  page.on("pageerror", (error) => pageErrors.push(error.message));
  try {
    console.log("Open Exposed and focus search with the slash shortcut.");
    await page.goto("/?strictness=1");
    await page.waitForLoadState("networkidle");
    await expect(
      page.getByRole("link", { name: "Exposed home" }),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "Start with a name" }),
    ).toBeVisible();
    await page.screenshot({ path: `${evidence}/before.png`, fullPage: true });
    await writeFile(
      `${evidence}/before.aria.txt`,
      await page.locator("body").ariaSnapshot(),
    );
    await page.keyboard.press("/");
    const input = page.getByRole("searchbox", {
      name: "Search MPs and funders",
    });
    await expect(input).toBeFocused();

    console.log(
      "Enter a two-character query and submit; search must stay idle.",
    );
    await input.fill("Jo");
    await input.press("Enter");
    await expect(
      page.getByText("Type 1 more character to search."),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "Start with a name" }),
    ).toBeVisible();
    assert.deepEqual(requests, []);

    console.log(
      "Type John and compare rendered names with the real API response.",
    );
    const bodyText = await readFile(responseFile, "utf8");
    await input.fill("  John  ");
    await writeFile(`${evidence}/response.json`, bodyText);
    const body = JSON.parse(bodyText);
    assert.deepEqual(
      body.entities
        .map(({ name, kind, funder_kind }) => ({ name, kind, funder_kind }))
        .sort((a, b) => a.name.localeCompare(b.name)),
      [
        { name: "John Humphries", kind: "MP", funder_kind: null },
        { name: "John McDonnell", kind: "MP", funder_kind: null },
      ],
    );
    assert(
      body.entities.every(
        (entity) =>
          typeof entity.id === "string" && /^[1-9]\d*$/u.test(entity.id),
      ),
      "Search MPs have stable Parliament IDs",
    );
    await expect(
      page.getByRole("heading", { name: "2 results shown" }),
    ).toBeVisible();
    await expect(page.locator(".entity-name")).toHaveText(
      body.entities.map((entity) => entity.name),
    );
    await expect(
      page.getByRole("button", { name: "MP", exact: true }),
    ).toHaveCount(2);
    await expect(page.locator(".entity-name mark")).toHaveText([
      "John",
      "John",
    ]);
    assert.equal(new URL(page.url()).searchParams.get("q"), "John");
    await page.screenshot({ path: `${evidence}/results.png`, fullPage: true });
    await writeFile(
      `${evidence}/results.aria.txt`,
      await page.locator("body").ariaSnapshot(),
    );

    console.log(
      "Click Clear search and verify the idle state, focus, and URL.",
    );
    await page.getByRole("button", { name: "Clear search" }).click();
    await expect(input).toHaveValue("");
    await expect(input).toBeFocused();
    await expect(
      page.getByRole("heading", { name: "Start with a name" }),
    ).toBeVisible();
    await expect(page.locator(".entity-row")).toHaveCount(0);
    assert.equal(new URL(page.url()).searchParams.has("q"), false);
    assert.deepEqual(pageErrors, []);
    assert.deepEqual(
      backendRequests,
      [],
      "Browser requests only the SvelteKit frontend",
    );
    assert(
      requests.some((url) => new URL(url).searchParams.get("q") === "John"),
      "Search uses SvelteKit page data",
    );
    await page.screenshot({ path: `${evidence}/cleared.png`, fullPage: true });
    await writeFile(
      `${evidence}/cleared.aria.txt`,
      await page.locator("body").ariaSnapshot(),
    );
    await writeFile(
      `${evidence}/proof.json`,
      JSON.stringify(
        {
          feature: "search-controls",
          entryPoints: [
            "slash shortcut",
            "short-query Enter",
            "typed search",
            "Clear search button",
          ],
          baseURL: origin.origin,
          requests,
          backendRequests,
          entities: body.entities,
          pageErrors,
          result: "passed",
        },
        null,
        2,
      ),
    );
    console.log(`Search controls passed. Evidence: ${evidence}`);
  } catch (error) {
    await writeFile(`${evidence}/failure.txt`, String(error.stack ?? error));
    await page.screenshot({ path: `${evidence}/failure.png`, fullPage: true });
    throw error;
  } finally {
    await context.tracing.stop({ path: `${evidence}/trace.zip` });
  }
} finally {
  await browser.close();
}
