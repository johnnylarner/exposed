export function overviewFixture() {
  return {
    current_term_start: "2024-07-09T00:00:00Z",
    member_count: 8,
    unavailable_member_count: 1,
    incomplete: true,
    quartiles: [
      {
        quartile: 1,
        member_count: 1,
        minimum: "0",
        maximum: "0",
        incomplete_member_count: 0,
      },
      {
        quartile: 2,
        member_count: 2,
        minimum: "1200",
        maximum: "4000",
        incomplete_member_count: 0,
      },
      {
        quartile: 3,
        member_count: 2,
        minimum: "15000",
        maximum: "28000",
        incomplete_member_count: 1,
      },
      {
        quartile: 4,
        member_count: 2,
        minimum: "65000",
        maximum: "150000",
        incomplete_member_count: 0,
      },
    ],
    top_members: [
      "Alex Morgan",
      "Sam Taylor",
      "Robin Patel",
      "Jamie Williams",
      "Charlie Smith",
    ].map((name, i) => ({
      member: {
        id: String(i + 1),
        name,
        party_name: "Example Party",
        party_id: 1,
        membership_from: "Richmond and Northallerton",
        is_current_commons: true,
      },
      total: {
        amount: ["150000", "65000", "28000", "15000", "4000"][i],
        declaration_count: 4,
        incomplete: i === 2,
      },
    })),
    parties: [
      {
        party_id: 1,
        party_name: "Labour",
        member_count: 4,
        total: { amount: "180000", declaration_count: 12, incomplete: false },
      },
      {
        party_id: 2,
        party_name: "Conservative",
        member_count: 3,
        total: { amount: "78000", declaration_count: 9, incomplete: true },
      },
      {
        party_id: 3,
        party_name: "Independent",
        member_count: 1,
        total: { amount: null, declaration_count: 1, incomplete: true },
      },
    ],
  };
}
