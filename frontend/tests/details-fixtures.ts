export const funderId = "10000000-0000-0000-0000-000000000001";
export const member = {
  id: "1",
  name: "Ava Example",
  party_id: 1,
  party_name: "Party A",
  membership_from: "Example constituency",
  is_current_commons: true,
};
export const memberHistoryFixture = {
  member,

  declarations: [
    {
      source_id: 120,
      category_id: 3,
      category_name: "Support",
      registered_at: "2026-01-21T00:00:00Z",
      entries: [
        {
          funder: { id: funderId, name: "Exact Funder" },
          amount: "9007199254740993.123456789",
          currency: "GBP",
          payment_type: "Cash",
        },
        {
          funder: { id: funderId, name: "Exact Funder" },
          amount: "9007199254740993.123456789",
          currency: "GBP",
          payment_type: "Cash",
        },
        { funder: null, amount: null, currency: null, payment_type: null },
      ],
    },
    {
      source_id: 119,
      category_id: 987,
      category_name: "Visits",
      registered_at: null,
      entries: [],
    },
  ],
};
export function declaredTotals(
  currentTerm: {
    amount: string | null;
    declaration_count: number;
    incomplete: boolean;
  },
  historical = currentTerm,
  currentYear = currentTerm,
  calendarYear = 2026,
) {
  return {
    calendar_year: calendarYear,
    current_year: currentYear,
    current_term_start: "2024-07-09T00:00:00Z",
    current_term: currentTerm,
    historical,
  };
}
export const emptyDeclaredTotals = declaredTotals({
  amount: null,
  declaration_count: 0,
  incomplete: false,
});
export const memberDetails = {
  ...memberHistoryFixture,
  summary: {
    declared_totals: declaredTotals(
      {
        amount: "18014398509481986.246913578",
        declaration_count: 1,
        incomplete: true,
      },
      {
        amount: "18014398509481986.246913578",
        declaration_count: 2,
        incomplete: true,
      },
    ),
    entry_count: 3,
    unknown_currency_count: 1,
    first_registered_at: "2026-01-21T00:00:00Z",
    last_registered_at: "2026-01-21T00:00:00Z",
    undated_declaration_count: 1,
    currencies: [
      {
        currency: "GBP",
        amount: "18014398509481986.246913578",
        entry_count: 2,
        unknown_amount_count: 0,
        categories: [
          {
            category_id: 3,
            category_name: "Support",
            amount: "18014398509481986.246913578",
            entry_count: 2,
            unknown_amount_count: 0,
          },
          {
            category_id: 987,
            category_name: "Unrecognized source category",
            amount: "-5",
            entry_count: 1,
            unknown_amount_count: 0,
          },
        ],
      },
      {
        currency: "EUR",
        amount: null,
        entry_count: 1,
        unknown_amount_count: 1,
        categories: [
          {
            category_id: 4,
            category_name: "Gifts",
            amount: null,
            entry_count: 1,
            unknown_amount_count: 1,
          },
        ],
      },
      {
        currency: "JPY",
        amount: "0",
        entry_count: 1,
        unknown_amount_count: 0,
        categories: [
          {
            category_id: 4,
            category_name: "Gifts",
            amount: "0",
            entry_count: 1,
            unknown_amount_count: 0,
          },
        ],
      },
    ],
  },
};
export const funderDetails = {
  funder: {
    id: funderId,
    name: "Exact Funder",
    funder_kind: "Company",
    company_number: "00123456",
    aliases: [" Exact Funder ", "EXACT FUNDER", "Exact Funder"],
  },
  declarations: [
    {
      member,
      declaration: {
        ...memberHistoryFixture.declarations[0],
        entries: memberHistoryFixture.declarations[0].entries.slice(0, 2),
      },
    },
  ],
  entry_count: 7,
  unknown_currency_count: 2,
  recipient_limit: 10,
  currencies: [
    {
      currency: "EUR",
      amount: null,
      parties: [
        {
          party_id: 2,
          party_name: "Party B",
          amount: null,
          entry_count: 1,
          unknown_amount_count: 1,
        },
      ],
      top_recipients: [
        {
          member: {
            ...member,
            id: "2",
            name: "Bea Example",
            party_name: "Party B",
          },
          amount: null,
          entry_count: 1,
          unknown_amount_count: 1,
        },
      ],
    },
    {
      currency: "GBP",
      amount: "18014398509481981.246913578",
      parties: [
        {
          party_id: 1,
          party_name: "Party A",
          amount: "18014398509481986.246913578",
          entry_count: 3,
          unknown_amount_count: 1,
        },
        {
          party_id: 2,
          party_name: "Party B",
          amount: "-5",
          entry_count: 1,
          unknown_amount_count: 0,
        },
      ],
      top_recipients: [
        {
          member,
          amount: "18014398509481986.246913578",
          entry_count: 3,
          unknown_amount_count: 1,
        },
        {
          member: {
            ...member,
            id: "2",
            name: "Bea Example",
            party_name: "Party B",
          },
          amount: "0",
          entry_count: 1,
          unknown_amount_count: 0,
        },
      ],
    },
    {
      currency: "USD",
      amount: "500",
      parties: [
        {
          party_id: 2,
          party_name: "Party B",
          amount: "500",
          entry_count: 1,
          unknown_amount_count: 0,
        },
      ],
      top_recipients: [
        {
          member: {
            ...member,
            id: "2",
            name: "Bea Example",
            party_name: "Party B",
          },
          amount: "500",
          entry_count: 1,
          unknown_amount_count: 0,
        },
      ],
    },
  ],
};
