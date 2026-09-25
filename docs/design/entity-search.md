# Entity search

The first frontend slice helps a journalist find a known MP or funder before
investigating their registered financial interests. It uses the existing Rust
search endpoint. Entity pages and automated feedback collection are outside this
slice; feedback comes from manual testing.

## Design decisions

- Laptop first, with a usable narrow layout.
- A true black application base with restrained purple accents. Attio informed
  the functionality; the palette follows this project's brief.
- One prominent search field and a persistent list beneath it.
- MPs and funders stay mixed in backend similarity order. Left-hand icons
  distinguish the roles: a Parliament icon for MPs and a coin for every funder.
  Both use identical purple colouring, backgrounds and borders for equal
  emphasis. A small legend explains the shapes. Hover, keyboard focus or tap
  reveals the role and funder subtype; Escape dismisses it. Accessible labels
  retain this context for screen readers.
- Funder kind appears as a compact purple tag containing both an icon and
  always-visible text. Person represents Individual, building represents Company,
  group represents Trade union, question mark represents Unclassified, and a tag
  represents other kinds. The text uses the API's existing `funder_kind` field;
  missing or unspecified kinds display as "Unclassified". Tags wrap beneath
  names on narrow screens, keeping the full text readable.
- Search updates after a short typing pause. A labelled similarity slider
  lets users broaden matches without changing their query.
- Preserve source spelling and casing, including lowercase funder names. There
  are no portraits, parties, constituencies or recency claims without API data.

## Visual system

| Token | Value | Role |
| --- | --- | --- |
| Black | `#000000` | Application canvas |
| Raised | `#111111` | Search field |
| Pale | `#F0EDF3` | Primary text |
| Muted | `#ADA6B4` | Secondary text |
| Purple | `#B67BE5` | Focus and interactive accents |
| Divider | `#28232E` | Result boundaries |

Manrope is bundled locally as a variable font. Its open shapes keep long names
legible; weight and spacing distinguish the title, names and supporting text.
The desktop type scale is 38 / 29 / 17 / 15 / 13 / 11 pixels. The search column
is 800 pixels wide, horizontally centred, with its contents left aligned.

```text
exposed                                          UK Parliament

          Search MPs and funders
          Register context
          [ Search by name                           / ]

          Similarity threshold                    1.00
          [ Broader ------------------------- Closer ]

          Results shown                 Best matches first
          [role icon] Entity name
          [role icon] Entity name
          [role icon] Entity name
```

Review against the brief: the characteristic content is people's and
organisations' names. The results are aligned rows, with colour reserved for
focus, matching text and small identifying elements. No financial totals or
activity dashboard are implied by a name-only API. The earlier colour study was
corrected from coloured backgrounds to black with coloured accents.

## Interaction and testing

- Three characters, a 250 ms debounce, and ten results per entity type initially.
- The similarity threshold runs from 0 to 1 in steps of 0.01, defaulting to 1.
  Lower values include less similar names; 1 does not mean an exact full name.
  Threshold changes use the same debounce and cancellation as typing.
  Query and threshold are preserved in the URL, including on reload and history
  navigation. Clearing the query keeps the chosen threshold. Invalid URL
  thresholds fall back to the default.
- The backend determines similarity and order; substring highlighting is only
  visual. Original spelling and duplicate-looking entries remain intact.
- Superseded requests are cancelled and cannot replace current results.
- Loading, short queries, no matches, malformed responses, timeouts and service
  failures have explicit states. Retry does not require retyping the query.
- `/` focuses the search, Escape clears it, and query URLs support manual review.
- Names and queries are rendered as text. Keyboard focus, live announcements,
  narrow layouts and reduced motion are supported.

Frontend development is a separate Svelte/Vite Compose service, proxying the
existing API through the same origin. No database or Rust changes are needed.
