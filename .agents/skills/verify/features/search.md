# Canonical search and alias matches

Searching a recorded name or alias returns the canonical funder once, with the
name that supplied the winning match recorded in `match_source`.

## Sub-features

- `unite` returns Unite the Union once, alongside broader matches at strictness 0.8.
- `west mid` returns Unite through `UNITE The Union (West Midlands)` at strictness 0.8, ahead of the other accepted West Midlands matches.
- `hsbc` returns one HSBC UK Bank plc entity at strictness 0.8.
- `jahn` and `jahn mcdonnell` recover John McDonnell through people-search fallback when no primary people match.

## How to get to it (user POV)

Enter a term in the landing-page search box. Set the similarity slider to the
regression default of 0.8. The browser sends the same `/search` request checked here.
The public API also accepts GET requests with `term`, `strictness`, and
`max_entries` query parameters.

## Driving it with HTTP

Preconditions: build the current API and select the read-only snapshot using the verify skill.

- Run `python3 scripts/verify-search-regression.py "$evidence_dir"` once for the whole map, using a new evidence directory.
- Require `PASS unite`, `PASS west-mid`, and `PASS hsbc`.
- Inspect their `.request.txt`, `.response.json`, and `.diff.txt` artifacts. Each entity array must match the baseline after omitting IDs, including order, names, kinds, and match provenance.

Run the opt-in people-search HTTP regression against the same read-only snapshot:

```sh
SEARCH_REGRESSION_DATABASE_URL="$DATABASE_URL" cargo test --package exposed --test integration loaded_data_people_search_regression -- --ignored
```

Require the test to pass. It checks that John McDonnell remains first for correct
spelling and appears within the normal result limit for both misspelled queries.

## Gotchas

- The frontend and regression searches default to strictness 0.8.
- All cases use the frontend's current limit of ten results per entity type.
- A frozen database tests search deduplication by persisted ID, not upstream observation resolution.
- This HTTP check does not prove slider behavior or rendering in the browser.
