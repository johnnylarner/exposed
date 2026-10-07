# Funding roles

Cleaning separates payment occurrences from the donor, payer, and ultimate-payer observations attached to them.

## Sub-features

- `roles-distinct` preserves different donor and ultimate-payer names as separate observations for one payment.
- `roles-identical` preserves identical names as separate observations with distinct IDs.
- `roles-repeated` preserves two identical payment occurrences and their observations.
- `roles-three` links donor, payer, and ultimate payer through their corresponding funding columns.
- `roles-declaration` retains a declaration-level payer with no funding entry.
- `roles-metadata` retains donor metadata on the donor observation only, including an unnamed donor.

## How to get to it (user POV)

Run `exposed data declarations clean config.yaml --ingestion-key UUID` against a completed capture. Open the two files in `data/UUID/cleaned/declarations/` and join each role reference to `funders.funder_id`.

## Driving it with declarations.py

Preconditions:

- Launch and Doctor passed for `$VERIFY_EVIDENCE`.

- **Clean the fixture.** Run `"$VERIFY_EVIDENCE/venv/bin/python" exposed/.agents/skills/verify/scripts/declarations.py drive "$VERIFY_EVIDENCE"` from the repository root.
- **Inspect different names and repeated payments.** Declaration 1 has two funding rows. Each links to a `Sir Trevor Chinn` donor and a `Labour Together Limited` ultimate payer. All four funder IDs differ.
- **Inspect identical names.** Declaration 2 has one funding row and two `Sir Trevor Chinn` observations. Their roles and IDs differ, and each funding reference selects the correct one.
- **Inspect all roles.** Declarations 3 through 11 each have three observations, one per role. Their name features agree within each declaration.
- **Inspect declaration scope.** Declaration 20 has a payer observation with null `funding_entry_id` and no funding row.
- **Inspect missing-name metadata.** Declaration 21 has a donor observation with null `name_raw`, `missing` status, and company number `00001234`.

The helper asserts these results in the saved Parquet tables and records `funding-roles` in `proof.json`.

## Gotchas

- A funder row is an observation, not a resolved entity. Identical names must not collapse.
- Donor metadata does not determine which name-cleaning rules run.
- Grouped donors use `Name` in this fixture. The top-level payment uses `DonorName`.
- Absent roles stay absent unless donor metadata supplies an observation.
