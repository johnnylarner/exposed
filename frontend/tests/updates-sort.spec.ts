import { test, expect } from "@playwright/test";
import { sortUpdates, type EntityUpdate } from "../src/lib/updates";

test("sorting preserves nanoseconds and alphabetical ties in both directions", () => {
  const row = (
    id: string,
    name: string,
    latestRegistration: string,
  ): EntityUpdate => ({ id, name, latestRegistration, declarationCount: 1 });
  const rows = Object.freeze([
    row("9", "Zebra", "2026-10-10T00:00:00Z"),
    row("4", "alpha", "2026-10-10T00:00:00.000000000Z"),
    row("3", "Alpha", "2026-10-10T00:00:00.0Z"),
    row("2", "Alpha", "2026-10-10T00:00:00Z"),
    row("8", "Earlier", "2026-10-09T23:59:59.999999999Z"),
    row("7", "Later", "2026-10-10T00:00:00.000000001Z"),
    row("6", "Latest", "2026-10-10T00:00:00.000000002Z"),
  ]);
  expect(sortUpdates(rows, "newest").map((row) => row.id)).toEqual([
    "6",
    "7",
    "2",
    "3",
    "4",
    "9",
    "8",
  ]);
  expect(sortUpdates(rows, "oldest").map((row) => row.id)).toEqual([
    "8",
    "2",
    "3",
    "4",
    "9",
    "7",
    "6",
  ]);
  expect(rows.map((row) => row.id)).toEqual([
    "9",
    "4",
    "3",
    "2",
    "8",
    "7",
    "6",
  ]);
  expect(sortUpdates([], "newest")).toEqual([]);
});
