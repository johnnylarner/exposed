# Canonical search and alias matches

Searching a recorded name or alias returns the canonical funder once, with the
name that supplied the winning match recorded in `match_source`.

## Sub-features

- `unite` returns Unite the Union once, alongside broader matches at strictness 0.8.
- `west mid` returns Unite through `UNITE The Union (West Midlands)` at strictness 0.8, ahead of the other accepted West Midlands matches.
- `hsbc` returns one HSBC UK Bank plc entity at strictness 0.8.

## How to get to it (user POV)

Enter a term in the landing-page search box. Set the similarity slider to the
regression default of 0.8. The browser sends the same `/search` request checked here.
The public API also accepts GET requests with `term`, `strictness`, and
`max_entries` query parameters.

## Driving it with HTTP

Preconditions: launch the snapshot API using the verify skill.

- Run `./scripts/check-search-regression.py "$base_url" "$evidence_dir/http"`.
- Require `PASS unite`, `PASS west-mid`, and `PASS hsbc`.
- Inspect their `.request.txt`, `.response.json`, and `.diff.txt` artifacts. Each entity array must match the baseline after omitting IDs, including order, names, kinds, and match provenance.

## Gotchas

- The default strictness for all regression searches is 0.8. The frontend currently initializes its slider to 1.0, so set it to 0.8 for these checks.
- All cases use the frontend's current limit of ten results per entity type.
- A frozen database tests search deduplication by persisted ID, not upstream observation resolution.
- This HTTP check does not prove slider behavior or rendering in the browser.
