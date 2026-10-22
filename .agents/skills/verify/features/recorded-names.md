# Recorded funder names

The Unite and HSBC detail profiles retain the accepted canonical name and every
recorded alias, including spelling and capitalization.

## Sub-features

- `unite-recorded-names` retains Unite the Union and its thirteen aliases.
- `hsbc-recorded-names` retains HSBC UK Bank plc and its three aliases.

## How to get to it (user POV)

Search for `unite` or `hsbc` and open the canonical result's funder detail page.
The public API exposes the profile and aliases through `/funders/{UUID}`.

## Driving it with HTTP

Preconditions: launch the snapshot API using the verify skill.

- Run `./.agents/skills/verify/scripts/check-search-regression.py "$base_url" "$evidence_dir/http"` once for the whole map.
- Require `PASS unite-recorded-names` and `PASS hsbc-recorded-names`.
- Compare each response's `funder` object with the independent JSON expectation. Canonical names, funder kinds, company numbers, and the exact alias arrays must remain unchanged.

## Gotchas

- IDs are arbitrary. Follow the current search result to fetch details and omit IDs from comparisons.

- A renamed, added, removed, reordered, or differently capitalized alias fails this check.
- Updating a frozen expectation to match a changed result would erase the regression signal. First identify whether code or snapshot data changed.
- Funding totals and recipients are unrelated to recorded names and are not compared.
