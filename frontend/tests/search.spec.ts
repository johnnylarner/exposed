import { expect, test } from "./backend";

test("live search preserves API ranking, spelling, duplicates and entity context", async ({
  backend,
  page,
}) => {
  const entities = [
    {
      id: "10000000-0000-0000-0000-000000000001",
      name: "john lewis partnership",
      kind: "Funder",
      funder_kind: "Company",
      match_source: { kind: "name" },
    },
    {
      id: "1",
      name: "John Cooper",
      kind: "MP",
      funder_kind: null,
      match_source: { kind: "name" },
    },
    {
      id: "10000000-0000-0000-0000-000000000001",
      name: "John Kinder",
      kind: "Funder",
      funder_kind: "Individual",
      match_source: { kind: "name" },
    },
    {
      id: "10000000-0000-0000-0000-000000000001",
      name: "john lewis partnership",
      kind: "Funder",
      funder_kind: "Company",
      match_source: { kind: "name" },
    },
    {
      id: "10000000-0000-0000-0000-000000000001",
      name: "John Union",
      kind: "Funder",
      funder_kind: "Trade Union",
      match_source: { kind: "name" },
    },
    {
      id: "10000000-0000-0000-0000-000000000001",
      name: "John Trust",
      kind: "Funder",
      funder_kind: "Not Specified",
      match_source: { kind: "name" },
    },
    {
      id: "10000000-0000-0000-0000-000000000001",
      name: "John Foundation",
      kind: "Funder",
      funder_kind: null,
      match_source: { kind: "name" },
    },
  ];
  const requests: URL[] = [];
  await backend.route("**/api/search?*", async (route) => {
    requests.push(new URL(route.request().url()));
    await route.fulfill({ json: { entities } });
  });
  await page.goto("/search");
  const input = page.getByRole("searchbox");
  await input.fill("jo");
  await expect(
    page.getByText("Type 1 more character to search."),
  ).toBeVisible();
  await input.press("Enter");
  expect(requests).toHaveLength(0);
  await input.fill("  John  ");
  await expect(
    page.getByRole("heading", { name: "7 results shown" }),
  ).toBeVisible();
  expect(requests).toHaveLength(1);
  expect(requests[0].searchParams.get("term")).toBe("John");
  expect(requests[0].searchParams.get("max_entries")).toBe("10");
  expect(requests[0].searchParams.get("strictness")).toBe("0.8");
  await expect(page.locator(".entity-name")).toHaveText(
    entities.map((entity) => entity.name),
  );
  await expect(page.locator(".entity-kind-label")).toHaveText([
    "Company",
    "Individual",
    "Company",
    "Trade union",
    "Unclassified",
    "Unclassified",
  ]);
  await expect(page.locator(".entity-kind-label").first()).toBeVisible();
  await expect(
    page.locator(".entity-row").nth(1).locator(".entity-kind"),
  ).toHaveCount(0);
  expect(
    await page
      .locator(".entity-indicator:not(.entity-kind) .entity-icon")
      .evaluateAll((icons) =>
        icons.map((icon) => icon.getAttribute("aria-label")),
      ),
  ).toEqual([
    "Funder: Company",
    "MP",
    "Funder: Individual",
    "Funder: Company",
    "Funder: Trade union",
    "Funder: Unclassified",
    "Funder: Unclassified",
  ]);
  const individual = page.getByRole("button", {
    name: "Funder: Individual",
    exact: true,
  });
  await individual.focus();
  await expect(page.locator(".entity-tooltip")).toHaveText(
    "Funder: Individual",
  );
  await individual.press("Escape");
  await expect(page.locator(".entity-tooltip")).toHaveCount(0);
  await individual.blur();
  await individual.hover();
  await expect(page.locator(".entity-tooltip")).toHaveText(
    "Funder: Individual",
  );
  await page.locator(".entity-tooltip").hover();
  await expect(page.locator(".entity-tooltip")).toBeVisible();
  await input.hover();
  await expect(page.locator(".entity-tooltip")).toHaveCount(0);
  await expect(page.locator("mark").first()).toHaveText("john");
  await expect(page).toHaveURL(/\/search\?q=John$/u);
  await expect(page.getByRole("slider")).toHaveCount(0);
});

test("a late response cannot replace a newer query or repopulate a cleared search", async ({
  backend,
  page,
}) => {
  let releaseOld: () => void = () => {};
  let finishOld: () => void = () => {};
  const oldResponse = new Promise<void>((resolve) => {
    releaseOld = resolve;
  });
  const oldHandled = new Promise<void>((resolve) => {
    finishOld = resolve;
  });
  await backend.route("**/api/search?*", async (route) => {
    const term = new URL(route.request().url()).searchParams.get("term");
    if (term === "John") await oldResponse;
    await route.fulfill({
      json: {
        entities: [
          {
            id: "10000000-0000-0000-0000-000000000001",
            name: term === "John" ? "John Cooper" : "hsbc uk bank plc",
            kind: "Funder",
            funder_kind: "Company",
            match_source: { kind: "name" },
          },
        ],
      },
    });
    if (term === "John") finishOld();
  });
  await page.goto("/search");
  const input = page.getByRole("searchbox");
  const oldRequest = backend.waitForRequest((request) =>
    request.url().includes("term=John"),
  );
  await input.fill("John");
  await oldRequest;
  await input.fill("HSBC");
  await expect(page.locator(".entity-name")).toHaveText("hsbc uk bank plc");
  releaseOld();
  await oldHandled;
  await expect(page.locator(".entity-name")).toHaveText("hsbc uk bank plc");
  await expect(page.getByText("John Cooper", { exact: true })).toHaveCount(0);
  await page.getByRole("button", { name: "Clear search" }).click();
  await expect(input).toHaveValue("");
  await expect(
    page.getByRole("heading", { name: "Start with a name" }),
  ).toBeVisible();
  await expect(page.locator(".entity-row")).toHaveCount(0);
  await expect(input).toBeFocused();
  await expect(page).not.toHaveURL(/q=/);
  expect(new URL(page.url()).pathname).toBe("/search");
});

test("an unavailable service offers retry and an empty response offers a useful next step", async ({
  backend,
  page,
}) => {
  let calls = 0;
  await backend.route("**/api/search?*", async (route) => {
    calls += 1;
    if (calls === 1)
      await route.fulfill({ status: 503, body: "service unavailable" });
    else await route.fulfill({ json: { entities: [] } });
  });
  await page.goto("/search?q=unknown");
  await expect(page.getByRole("alert")).toContainText("Search couldn’t load");
  await page.getByRole("button", { name: "Try again" }).click();
  await expect(
    page.getByRole("heading", { name: "No matches for “unknown”" }),
  ).toBeVisible();
  await expect(
    page.getByText(
      "Try a surname, a shorter company name, or a different spelling.",
    ),
  ).toBeVisible();
  await page.getByRole("button", { name: "Edit your search" }).click();
  await expect(page.getByRole("searchbox")).toBeFocused();
  expect(calls).toBe(2);
});

test("search text and names containing markup or regular expression syntax remain literal", async ({
  backend,
  page,
}) => {
  const name = 'A.* <img src=x onerror="alert(1)"> & Co';
  await backend.route("**/api/search?*", (route) =>
    route.fulfill({
      json: {
        entities: [
          {
            id: "10000000-0000-0000-0000-000000000001",
            name,
            kind: "Funder",
            funder_kind: "Company",
            match_source: { kind: "name" },
          },
        ],
      },
    }),
  );
  await page.goto("/search");
  await page.getByRole("searchbox").fill("A.*");
  await expect(page.locator(".entity-name")).toHaveText(name);
  await expect(page.locator(".entity-name mark")).toHaveText("A.*");
  await expect(page.locator(".entity-name img")).toHaveCount(0);
});

test("keyboard search and long results remain usable on a narrow screen", async ({
  backend,
  page,
}) => {
  await page.setViewportSize({ width: 360, height: 780 });
  await backend.route("**/api/search?*", (route) =>
    route.fulfill({
      json: {
        entities: [
          {
            id: "10000000-0000-0000-0000-000000000001",
            name: "Association of Research and Public Interest Organisations of the United Kingdom",
            kind: "Funder",
            funder_kind: "Unincorporated association",
            match_source: { kind: "name" },
          },
        ],
      },
    }),
  );
  await page.goto("/search");
  await page.waitForLoadState("networkidle");
  await page.keyboard.press("/");
  await expect(page.getByRole("searchbox")).toBeFocused();
  await page.getByRole("searchbox").fill("Association");
  await expect(
    page.getByRole("button", { name: "Funder: Unincorporated association" }),
  ).toBeVisible();
  await expect(page.locator(".entity-kind-label")).toHaveText(
    "Unincorporated association",
  );
  await expect(page.locator(".entity-kind-label")).toBeVisible();
  await expect(page.locator(".entity-kind")).toBeVisible();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await page.keyboard.press("Escape");
  await expect(page.getByRole("searchbox")).toHaveValue("");
});

test("alias provenance preserves canonical navigation and escapes highlighted alias text on mobile", async ({
  backend,
  page,
}) => {
  const id = "10000000-0000-0000-0000-000000000009";
  const alias =
    'West Midlands <img src=x onerror="alert(1)"> ' + "Union".repeat(80);
  await page.setViewportSize({ width: 360, height: 780 });
  await backend.route("**/api/search?*", (route) =>
    route.fulfill({
      json: {
        entities: [
          {
            id,
            name: "Unite the Union",
            kind: "Funder",
            funder_kind: "Trade Union",
            match_source: { kind: "alias", name: alias },
          },
          {
            id: "1",
            name: "Westminster Member",
            kind: "MP",
            funder_kind: null,
            match_source: { kind: "name" },
          },
        ],
      },
    }),
  );
  await page.goto("/search?q=West&strictness=0.5");
  const link = page.locator(".entity-name").first();
  await expect(link).toHaveText("Unite the Union");
  await expect(link).toHaveAttribute("href", `/funders/${id}?q=West`);
  await expect(link).toHaveAccessibleDescription(`Matched alias: ${alias}`);
  await expect(page.locator(".entity-alias")).toHaveText(
    `Matched alias: ${alias}`,
  );
  await expect(page.locator(".entity-alias mark")).toHaveText("West");
  await expect(page.locator(".entity-alias img")).toHaveCount(0);
  await expect(page.locator(".entity-alias")).toHaveCount(1);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
});

for (const platform of ["MacIntel", "Win32"]) {
  const mac = platform === "MacIntel";
  const modifier = mac ? "Meta" : "Control";
  const modifiers = { metaKey: mac, ctrlKey: !mac };

  test(`${platform} search shortcut focuses search and preserves its query`, async ({
    backend,
    page,
  }) => {
    await page.addInitScript((platform) => {
      Object.defineProperty(navigator, "platform", { value: platform });
    }, platform);
    await backend.route("**/api/search?*", (route) =>
      route.fulfill({ json: { entities: [] } }),
    );
    await page.goto("/search");
    const input = page.getByRole("searchbox");
    const otherControl = page.getByRole("link", { name: "Exposed home" });
    await expect(page.locator(".search-shortcut")).toHaveText(
      mac ? "⌘ K" : "Ctrl K",
    );
    await otherControl.focus();
    await page.keyboard.press(`${modifier}+k`);
    await expect(input).toBeFocused();
    await input.fill("  John  ");
    await expect(
      page.getByRole("heading", { name: "No matches for “John”" }),
    ).toBeVisible();
    const url = page.url();
    await otherControl.focus();
    await page.keyboard.press(`${modifier}+k`);
    await expect(input).toBeFocused();
    await expect(input).toHaveValue("  John  ");
    expect(page.url()).toBe(url);
    await page.keyboard.press(`${modifier}+k`);
    await expect(input).toHaveValue("  John  ");
    expect(page.url()).toBe(url);

    for (const init of [
      { metaKey: !mac, ctrlKey: mac },
      { metaKey: true, ctrlKey: true },
      { ...modifiers, shiftKey: true },
      { ...modifiers, altKey: true },
      { ...modifiers, isComposing: true },
      {},
    ]) {
      await otherControl.focus();
      const prevented = await otherControl.evaluate((element, init) => {
        const event = new KeyboardEvent("keydown", {
          key: "k",
          bubbles: true,
          cancelable: true,
          ...init,
        });
        element.dispatchEvent(event);
        return event.defaultPrevented;
      }, init);
      expect(prevented).toBe(false);
      await expect(otherControl).toBeFocused();
    }

    await otherControl.focus();
    await otherControl.evaluate((element, modifiers) => {
      const event = new KeyboardEvent("keydown", {
        key: "k",
        bubbles: true,
        cancelable: true,
        ...modifiers,
      });
      event.preventDefault();
      element.dispatchEvent(event);
    }, modifiers);
    await expect(otherControl).toBeFocused();
    await input.dispatchEvent("compositionstart");
    await otherControl.focus();
    const composingPrevented = await otherControl.evaluate(
      (element, modifiers) => {
        const event = new KeyboardEvent("keydown", {
          key: "k",
          bubbles: true,
          cancelable: true,
          ...modifiers,
        });
        element.dispatchEvent(event);
        return event.defaultPrevented;
      },
      modifiers,
    );
    expect(composingPrevented).toBe(false);
    await expect(otherControl).toBeFocused();
    await expect(input).toHaveValue("  John  ");
    await input.dispatchEvent("compositionend");
    await page.keyboard.press(`${modifier}+k`);
    await expect(input).toBeFocused();
    await input.press("/");
    await expect(input).toHaveValue("  John  /");
    await otherControl.focus();
    await page.keyboard.press("Escape");
    await expect(input).toHaveValue("  John  /");
    await page.keyboard.press(`${modifier}+k`);
    await page.keyboard.press("Escape");
    await expect(input).toHaveValue("");
    await page.getByRole("link", { name: "Exposed home" }).focus();
    await page.waitForLoadState("networkidle");
    await page.keyboard.press("/");
    await expect(input).toBeFocused();
    await page.goto("/members/1");
    const detailPrevented = await page.evaluate((modifiers) => {
      const event = new KeyboardEvent("keydown", {
        key: "k",
        bubbles: true,
        cancelable: true,
        ...modifiers,
      });
      window.dispatchEvent(event);
      return event.defaultPrevented;
    }, modifiers);
    expect(detailPrevented).toBe(false);
    await expect(page.getByRole("searchbox")).toHaveCount(0);
  });
}

for (const replacement of ["", "Jo"]) {
  test(`a pending search cannot restore the URL after replacing it with ${JSON.stringify(replacement)}`, async ({
    page,
    backend,
  }) => {
    let release: () => void = () => {};
    const pending = new Promise<void>((resolve) => {
      release = resolve;
    });
    await backend.route("**/api/search?*", async (route) => {
      await pending;
      await route.fulfill({ json: { entities: [] } });
    });
    await page.goto("/search");
    const input = page.getByRole("searchbox");
    const started = backend.waitForRequest((request) =>
      request.url().includes("term=John"),
    );
    await input.fill("John");
    await started;
    if (replacement) await input.fill(replacement);
    else await page.getByRole("button", { name: "Clear search" }).click();
    await expect(page).not.toHaveURL(/q=John/);
    release();
    await page.waitForLoadState("networkidle");
    await expect(input).toHaveValue(replacement);
    await expect(page).not.toHaveURL(/q=John/);
    expect(new URL(page.url()).searchParams.get("q") ?? "").toBe(replacement);
    await expect(
      page.getByRole("heading", { name: "Start with a name" }),
    ).toBeVisible();
  });
}

for (const pathname of [
  "/",
  "/search",
  "/members/1",
  "/members/1/history",
  "/funders/10000000-0000-0000-0000-000000000001",
  "/funders/10000000-0000-0000-0000-000000000001/history",
]) {
  test(`legacy strictness redirects before loading ${pathname}`, async ({
    backend,
    request,
  }) => {
    const calls: string[] = [];
    await backend.route("**", (route) => {
      calls.push(route.request().url());
      return route.fulfill({ json: { entities: [] } });
    });
    const response = await request.get(
      `${pathname}?strictness=0.1&q=Nigel&strictness=1&keep=value`,
      { maxRedirects: 0 },
    );
    expect(response.status()).toBe(308);
    expect(response.headers().location).toBe(
      `${pathname === "/" ? "/search" : pathname}?q=Nigel&keep=value`,
    );
    expect(calls).toEqual([]);
    if (pathname === "/") {
      const canonical = await request.get(response.headers().location);
      expect(canonical.ok()).toBe(true);
      expect(calls).toHaveLength(1);
      expect(new URL(calls[0]).searchParams.get("strictness")).toBe("0.8");
    }
  });
}

test("malformed search data shows an error without rendering results", async ({
  backend,
  page,
}) => {
  await backend.route("**/api/search?*", (route) =>
    route.fulfill({
      json: {
        entities: [
          {
            id: "10000000-0000-0000-0000-000000000009",
            name: "Unite",
            kind: "Funder",
            funder_kind: "Trade Union",
            match_source: null,
          },
        ],
      },
    }),
  );
  await page.goto("/search?q=West");
  await expect(page.getByRole("alert")).toContainText(
    "unexpected match source",
  );
  await expect(page.locator(".entity-row")).toHaveCount(0);
});
