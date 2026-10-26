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
