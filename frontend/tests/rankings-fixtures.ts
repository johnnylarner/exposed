export const rankingFixture = {
  period: "current-term",
  current_term_start: "2024-07-09T00:00:00Z",
  members: Array.from({ length: 10 }, (_, index) => ({
    id: String(index + 1),
    name: `Member ${index + 1}`,
    context: "Example Party · Example constituency",
    total: {
      amount: index === 0 ? "9007199254740993.123456789" : String(10 - index),
      declaration_count: 2,
      incomplete: index === 0,
    },
  })),
  individuals: [
    {
      id: "10000000-0000-0000-0000-000000000001",
      name: "Example individual",
      total: { amount: "0", declaration_count: 1, incomplete: false },
    },
  ],
  companies: [
    {
      id: "10000000-0000-0000-0000-000000000002",
      name: "Example company",
      total: { amount: "-12.3456789", declaration_count: 3, incomplete: false },
    },
  ],
  parties: [
    {
      id: "1",
      name: "Example party",
      total: { amount: null, declaration_count: 2, incomplete: true },
    },
  ],
};

export function completeRankingFixture(count = 23) {
  const members = Array.from({ length: count }, (_, index) => ({
    ...rankingFixture.members[0],
    id: String(index + 1),
    name: `Member ${index + 1}`,
    total: {
      amount:
        index === 0 ? "9007199254740993.123456789" : String(count - index),
      declaration_count: 2,
      incomplete: index === 0,
    },
  }));
  return {
    ...rankingFixture,
    members,
    individuals: members.map((row, index) => ({
      ...row,
      id: `10000000-0000-0000-0000-${String(index + 1).padStart(12, "0")}`,
      name: `Individual ${index + 1}`,
    })),
    companies: members.map((row, index) => ({
      ...row,
      id: `20000000-0000-0000-0000-${String(index + 1).padStart(12, "0")}`,
      name: `Company ${index + 1}`,
    })),
    parties: members.map((row, index) => ({
      ...row,
      name: `Party ${index + 1}`,
    })),
  };
}
