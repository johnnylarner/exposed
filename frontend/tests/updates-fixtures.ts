export const updatesFixture = {
  month_start: "2026-10-01T00:00:00Z",
  month_end: "2026-11-01T00:00:00Z",
  members: Array.from({ length: 12 }, (_, index) => ({
    id: String(index + 1),
    name: `Member ${index + 1}`,
    declaration_count: index === 0 ? 2 : 1,
    latest_registration:
      index === 0 ? "2026-10-31T23:59:59Z" : "2026-10-01T00:00:00Z",
  })),
  companies: [
    {
      id: "10000000-0000-0000-0000-000000000001",
      name: "Preferred company",
      declaration_count: 3,
      latest_registration: "2026-10-31T23:59:59Z",
    },
  ],
};

export const browseUpdatesFixture = {
  ...updatesFixture,
  members: Array.from({ length: 27 }, (_, index) => ({
    id: String(index + 1),
    name:
      ["Zebra", "alpha", "Alpha", "Alpha"][index % 4] +
      (index < 4 ? "" : ` ${index}`),
    declaration_count: index + 1,
    latest_registration:
      index < 4
        ? "2026-10-15T12:00:00Z"
        : `2026-10-${String(1 + (index % 26)).padStart(2, "0")}T12:00:00.${String(index).padStart(9, "0")}Z`,
  })).reverse(),
  companies: Array.from({ length: 25 }, (_, index) => ({
    id: `10000000-0000-0000-0000-${String(index + 1).padStart(12, "0")}`,
    name: `Company ${index + 1}`,
    declaration_count: index + 1,
    latest_registration: `2026-10-${String(index + 1).padStart(2, "0")}T12:00:00Z`,
  })).reverse(),
};
