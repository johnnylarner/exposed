export const sourceInterest = {
  source_id: 10016,
  text: "Repeated holding",
  created_when: "2012-06-14T12:49:14",
  last_amended_when: null,
  deleted_when: null,
  is_correction: false,
  children: [],
};
export const lordsFixture = {
  member: {
    id: "2175",
    name: "Lord Example",
    party_name: "Crossbench",
    membership_from: "Life peer",
    membership_status: "Current Member",
    membership_end_date: null,
  },
  captured_at: "2026-11-12T12:00:00Z",
  categories: [
    {
      source_id: 55,
      name: "Category 5: Other interests",
      sort_order: 20,
      placement: "other",
      interests: [
        {
          ...sourceInterest,
          text: "Active parent",
          is_correction: true,
          children: [
            {
              ...sourceInterest,
              text: "Deleted child",
              deleted_when: "2026-11-12T23:45:00-09:00",
            },
          ],
        },
        {
          ...sourceInterest,
          text: "Deleted parent",
          deleted_when: "2026-10-02T00:00:00",
          children: [{ ...sourceInterest, text: "Active child" }],
        },
        { ...sourceInterest, text: "Interest ceased on 1 January 2026" },
      ],
    },
    {
      source_id: 1002,
      name: "Category 2: Shareholdings etc. (a)",
      sort_order: 10,
      placement: "shareholdings",
      interests: [
        {
          ...sourceInterest,
          text: "ARTIS (Europe) Ltd (research and risk modelling)",
          deleted_when: "2025-04-05T00:00:00",
          last_amended_when: "2025-01-01T01:00:00+02:00",
          is_correction: true,
          children: [
            { ...sourceInterest, text: "Boot (Henry) plc (construction)" },
          ],
        },
        sourceInterest,
      ],
    },
    {
      source_id: 888,
      name: null,
      sort_order: 5,
      placement: "other",
      interests: [
        {
          ...sourceInterest,
          text: '<img src=x onerror="alert(1)"> & Partners\nSecond line',
        },
        { ...sourceInterest, text: null, created_when: null },
      ],
    },
    {
      source_id: 1002,
      name: "Category 2: Shareholdings etc. (a)",
      sort_order: 10,
      placement: "shareholdings",
      interests: [sourceInterest],
    },
    {
      source_id: 66,
      name: "Nil",
      sort_order: 30,
      placement: "other",
      interests: [{ ...sourceInterest, text: "No registrable interests" }],
    },
  ],
};
