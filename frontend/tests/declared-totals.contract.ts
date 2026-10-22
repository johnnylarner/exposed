import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, test } from "./backend";

const output = process.env.EXPOSED_CONTRACT_OUTPUT_DIR;
if (!output) throw new Error("Missing fresh Rust HTTP response directory");
const expected: Record<
  string,
  {
    display: string[];
    declared_totals: {
      current_term: {
        amount: string | null;
        declaration_count: number;
        incomplete: boolean;
      };
      historical: {
        amount: string | null;
        declaration_count: number;
        incomplete: boolean;
      };
    };
  }
> = JSON.parse(
  readFileSync(
    new URL(
      "../../contracts/declared-totals-v1/expected.json",
      import.meta.url,
    ),
    "utf8",
  ),
);
const responses: Record<string, string> = Object.fromEntries(
  Object.keys(expected).map((id) => [
    id,
    readFileSync(join(output, `${id}.json`), "utf8"),
  ]),
);

for (const [id, contract] of Object.entries(expected)) {
  test(`real backend declared GBP contract renders member ${id}`, async ({
    backend,
    page,
  }) => {
    await backend.route(`**/api/members/${id}`, (route) =>
      route.fulfill({
        body: responses[id],
        contentType: "application/json",
      }),
    );
    await page.goto(`/members/${id}`);
    await expect(page.locator(".member-total")).toHaveText(contract.display);
    await expect(page.locator(".member-earnings-totals h2").first()).toHaveText(
      "Declared amounts this parliamentary term",
    );
    await expect(
      page.locator(".member-earnings-totals h2").last(),
    ).toContainText("Historical declared amounts");
    for (const [index, period] of [
      contract.declared_totals.current_term,
      contract.declared_totals.historical,
    ].entries()) {
      if (period.amount !== null && period.declaration_count > 0) {
        const figure = page.locator(".member-total").nth(index);
        await expect(figure).toHaveAttribute(
          "aria-label",
          index === 0
            ? "Declared amounts this parliamentary term in GBP"
            : "Historical declared amounts in GBP",
        );
        if (period.incomplete)
          await expect(figure).toHaveAttribute(
            "title",
            "Known GBP amounts only. Some amounts unavailable.",
          );
        else await expect(figure).not.toHaveAttribute("title");
      }
    }
  });
}

test("authoritative headlines survive contradictory declarations", async ({
  backend,
  page,
}) => {
  const body = record(JSON.parse(responses["7001"]));
  body.declarations = [];
  await backend.route("**/api/members/7001", (route) =>
    route.fulfill({ json: body }),
  );
  await page.goto("/members/7001");
  await expect(page.locator(".member-total")).toHaveText(
    expected["7001"].display,
  );
});

const corruptions: { name: string; path: string[]; value?: unknown }[] = [
  { name: "missing declared totals", path: [] },
  { name: "missing amount", path: ["current_term", "amount"] },
  { name: "missing period", path: ["historical"] },
  { name: "invalid amount", path: ["historical", "amount"], value: 333 },
  {
    name: "invalid count",
    path: ["historical", "declaration_count"],
    value: -1,
  },
  { name: "missing count", path: ["current_term", "declaration_count"] },
  {
    name: "invalid boolean",
    path: ["current_term", "incomplete"],
    value: "false",
  },
  { name: "missing boolean", path: ["historical", "incomplete"] },
  { name: "null start", path: ["current_term_start"], value: null },
  { name: "invalid start", path: ["current_term_start"], value: "not a date" },
  { name: "missing start", path: ["current_term_start"] },
];
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
function record(value: unknown): Record<string, unknown> {
  if (!isRecord(value)) throw new Error("Expected a contract response object");
  return value;
}
for (const { name, path, value } of corruptions) {
  test(`mandatory headline fields fail closed: ${name}`, async ({
    backend,
    page,
  }) => {
    const body: unknown = JSON.parse(responses["7001"]);
    let parent = record(record(body).summary);
    const keys = ["declared_totals", ...path];
    const key = keys.pop();
    if (key === undefined) throw new Error("Missing corruption field");
    for (const field of keys) parent = record(parent[field]);
    if (value === undefined) delete parent[key];
    else parent[key] = value;
    await backend.route("**/api/members/7001", (route) =>
      route.fulfill({ json: body }),
    );
    await page.goto("/members/7001");
    await expect(
      page.getByText("Details returned an unexpected response. Try again."),
    ).toBeVisible();
    await expect(page.locator(".member-total")).toHaveCount(0);
  });
}
