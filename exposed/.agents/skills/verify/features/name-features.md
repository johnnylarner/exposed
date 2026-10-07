# Name features

Cleaning preserves source names and derives parallel columns for later entity resolution. Every role receives the same rules.

## Sub-features

- `names-unicode` preserves originals while normalizing Unicode, whitespace, punctuation, and titles in feature columns.
- `names-accents` folds accents in a separate column while tokens retain them.
- `names-aliases` extracts explicit trading names, legal suffixes, and matching acronym expansions.
- `names-ambiguity` retains geographic qualifiers and flags conjunctions and unmatched parentheses.
- `names-status` preserves confidential and blank names with withheld or missing status.
- `names-role-parity` checks all name feature columns for equal treatment across roles.

## How to get to it (user POV)

Run `exposed data declarations clean config.yaml --ingestion-key UUID`. Inspect `funders.parquet` name columns and compare `name_raw` with the original role columns in `funding_entries.parquet`.

## Driving it with declarations.py

Preconditions:

- Launch and Doctor passed for `$VERIFY_EVIDENCE`.
- Read [name-cases.json](../name-cases.json) for literal expected values.

- **Clean the names.** Run `"$VERIFY_EVIDENCE/venv/bin/python" exposed/.agents/skills/verify/scripts/declarations.py drive "$VERIFY_EVIDENCE"` from the repository root.
- **Inspect original text.** Full-width letters, repeated whitespace, apostrophes, and accents remain in the original funding columns and `name_raw`.
- **Inspect matching features.** `Mr. James O'Brien-Smith` variants produce person core `james obrien smith`. `Friends of Sinn Féin Canada` loses the accent only in `name_accent_folded`.
- **Inspect aliases and acronyms.** The CSC case has alias `DXC Technology Ltd`. The ASLEF case retains the explicit acronym and its matching expansion.
- **Inspect ambiguity.** `BJD (GB) Limited` retains `gb` in its organisation core. `Alan Milburn & Ruth Briel` sets `has_conjunction` and stays one observation per role.
- **Inspect parity.** For each case, the helper compares all derived name columns across donor, payer, and ultimate payer, in addition to asserting literal expected values.

The helper records `name-features` in `proof.json`. Inspect `funders.json` or the saved Parquet file for each value.

## Gotchas

- Equality across roles alone is insufficient. The literal expectations catch a rule that breaks for every role at once.
- Both person and organisation columns exist regardless of donor kind.
- These cases prove selected rules. They do not cover every alias marker, title, suffix, or confidential placeholder.
