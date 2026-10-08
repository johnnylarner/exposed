# Declaration verification map

This map covers the offline declarations CLI. Read the [skill](../SKILL.md) for Launch, Doctor, Drive, and Cleanup.

## Baseline preconditions

- Use a dedicated worktree with a successful Launch and Doctor.
- Start each run with a fresh evidence directory. The two drives keep separate scratch data.
- Stop capture processes before cleaning real data. This verification uses synthetic, immutable capture files.

## Driving conventions

Run `scripts/declarations.py drive` for the cleaning features, and `scripts/resolution.py drive` for identity resolution, through the run's Python as shown in the skill. The CLI is short-lived and noninteractive, so each helper records subprocess output directly. There is no shared server to drive.

## Proof and skip reporting

Pair each command and its exit code with the saved Parquet rows. The cleaning `proof.json` and `resolution/proof.json` exist only after their assertions pass. Do not claim fetch, import, malformed-input handling, calibrated probabilities, or every possible spelling is covered by these fixtures.

## Features

- [Funding roles](funding-roles.md) proves separate observations and correct joins, including identical names and repeated payments.
- [Name features](name-features.md) proves shared cleaning across roles and preservation of original names.
- [Capture preservation](capture-preservation.md) proves both output tables, unchanged captures, and rejection of an existing destination.
- [Identity resolution](identity-resolution.md) proves company anchors, donor-name and statistical links, role limits, explicit ultimate-payer attribution, and immutable publication.
