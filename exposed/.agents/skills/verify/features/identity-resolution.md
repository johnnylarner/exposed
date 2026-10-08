# Identity resolution

The resolver assigns run-local funder identities and separately selects a reporting funder for every payment. Its Python worker scores candidate pairs with pinned Splink and DuckDB versions.

## Sub-features

- `company-anchor` links equal captured company numbers and keeps different numbers separate without a registry lookup.
- `donor-name` links donor observations with the same usable normalized name despite absent or conflicting addresses.
- `statistical-link` links non-company charity observations with exact names, exact numbered-street addresses, and a score of at least 0.999.
- `role-boundary` does not link an otherwise identical payer and ultimate payer using donor-name evidence.
- `attribution` selects the explicit ultimate payer when present, otherwise the donor in this fixture.
- `resolution-publication` keeps each funding occurrence and observation, resolves without raw input, records model metadata, and rejects overwrite without changing the bundle.

## How to get to it (user POV)

Run `exposed data declarations clean config.yaml --ingestion-key UUID`, then `exposed data declarations resolve config.yaml --ingestion-key UUID` with `resolution_python` pointing at an interpreter with the pinned package. Open `data/UUID/resolved/declarations/` and join `observation_resolution.funder_id` to `cleaned/declarations/funders.funder_id`. Join `payment_attribution.funding_entry_id` to `cleaned/declarations/funding_entries.funding_entry_id`.

## Driving it with resolution.py

Preconditions: Launch and both Doctors passed for `$VERIFY_EVIDENCE`.

Run `"$VERIFY_EVIDENCE/venv/bin/python" exposed/.agents/skills/verify/scripts/resolution.py drive "$VERIFY_EVIDENCE"` from the repository root. Inspect `resolution/proof.json`, copied Parquet bundles, JSON projections, and command logs.

- Declarations 1–4 have donor `Gary Lubner`; two lack addresses and two have different numbered-street addresses. All four share an identity with `donor_name_link` basis. Six accepted pair decisions retain non-threshold Splink scores.
- Declarations 5–6 have donor `Example Charity` and the same full address. They share a `statistical_link` identity with a score of at least 0.999.
- Declarations 7–8 have company number `00001234` and different addresses. Declaration 9 has the same name but number `00005678`. The first two share a `source_reported_company` identity; the third differs.
- Declaration 10 has donor `Alice` and an explicit `Grant Trust` ultimate payer. Attribution selects the ultimate payer. Declaration 11 has donor `Other Person` and `Grant Trust` payer; its attribution selects the donor. The identically named payer and ultimate payer remain separate without address evidence.
- The fixture has 11 funding occurrences and 13 funder observations. Raw capture is removed after cleaning. Resolution succeeds from the saved cleaned tables, and a repeat is rejected without changing any published file.

## Gotchas

- The company number is captured evidence, not Companies House verification.
- Equal donor names can merge different people. That is the current deliberate policy; this fixture proves behavior, not real-world identity accuracy.
- Splink probabilities are computed from a frozen, uncalibrated model. A high score here does not establish calibration on the full declaration corpus.
- The fixture does not cover parent-payer attribution, withheld names, fuzzy-name review, or every entity kind. Existing Rust policy tests and the real CLI integration test cover more cases.
