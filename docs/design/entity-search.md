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
- MPs and funders stay mixed in backend similarity order. Type labels supply
  context without regrouping, reranking or merging records.
- Search updates after a short typing pause. Tuning stays in code.
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

          Results shown                 Best matches first
          [icon] Entity name                         Type
          [icon] Entity name                         Type
          [icon] Entity name                         Type
```

Review against the brief: the characteristic content is people's and
organisations' names. The results are aligned rows, with colour reserved for
focus, matching text and small identifying elements. No financial totals or
activity dashboard are implied by a name-only API. The earlier colour study was
corrected from coloured backgrounds to black with coloured accents.

## Interaction and testing

- Three characters, a 250 ms debounce, and ten results per entity type initially.
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
