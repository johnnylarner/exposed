# Canonical search and alias matches

Searching a recorded name or alias returns the canonical funder once, with the
name that supplied the winning match recorded in `match_source`.

## Sub-features

- `unite` returns Unite the Union once, alongside broader matches at strictness 0.8.
- `west mid` returns Unite through `UNITE The Union (West Midlands)` at strictness 0.8, ahead of the other accepted West Midlands matches.
- `hsbc` returns one HSBC UK Bank plc entity at strictness 0.8.
- `jahn` and `jahn mcdonnell` recover John McDonnell through people-search fallback when there are neither primary MP matches nor qualifying individual funder matches.

## How to get to it (user POV)

Follow the landing page's search link to `/search` and enter a term. The frontend
uses fixed strictness 0.8 and a limit of ten results per entity type. Browser
navigation goes through SvelteKit server loads, which call the Rust API.
The public Rust API accepts GET `/search` requests with `term`, `strictness`, and
`max_entries` query parameters. Frontend URLs use `q` for the term and redirect
legacy URLs to remove `strictness`.

## Driving it with HTTP

Preconditions: launch the snapshot API using the verify skill.

- Run `./.agents/skills/verify/scripts/check-search-regression.py "$base_url" "$evidence_dir/http"`.
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
- Equal-score funder matches sort by UUID. A snapshot refresh can change their
  order and which results fit within the ten-result limit, even though IDs are
  omitted from the JSON comparison.
- This HTTP check does not prove browser interactions or rendering.
