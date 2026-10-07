# Capture preservation

Cleaning publishes two Parquet tables while retaining the captured input and refusing to overwrite existing cleaned output.

## Sub-features

- `capture-raw` leaves the source partition byte-for-byte unchanged.
- `capture-tables` publishes both funding and funder tables.
- `capture-repeat` rejects an existing destination without modifying either published table or the raw partition.
- `capture-cleanup` removes scratch data while retaining the proof.

## How to get to it (user POV)

Run `exposed data declarations clean config.yaml --ingestion-key UUID` once, then repeat the same command against the same ingestion.

## Driving it with declarations.py

Preconditions:

- Launch and Doctor passed for `$VERIFY_EVIDENCE`.

- **Clean and repeat.** Run `"$VERIFY_EVIDENCE/venv/bin/python" exposed/.agents/skills/verify/scripts/declarations.py drive "$VERIFY_EVIDENCE"` from the repository root. The helper invokes the same cleaning command twice.
- **Inspect the first command.** `clean.command.json` records exit code 0. Both files exist under `tables/`, and the original partition remains in `raw-input.parquet`.
- **Inspect the repeat.** `repeat.command.json` records a nonzero exit code, and `repeat.stderr.txt` includes `already exist`. The helper compares bytes before and after this attempt.
- **Inspect cleanup.** Run the skill's Cleanup block. The `scratch` directory and owned database container are absent. `proof.json`, the raw fixture, and both output files remain.

The helper records `capture-preservation`, `raw_unchanged`, and `repeat_rejected_without_changes` in `proof.json` only after the comparisons pass.

## Gotchas

- Use a new evidence directory for another attempt. Do not delete published output to make a failed check pass.
- This fixture does not simulate a concurrent fetch or a process crash during publication.
- The helper has no dry-run mode. It creates and removes its own scratch files.
