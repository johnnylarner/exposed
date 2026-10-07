# Declaration cleaning verification map

This map covers the offline declarations CLI. Read the [skill](../SKILL.md) for Launch, Doctor, Drive, and Cleanup.

## Baseline preconditions

- Use a dedicated worktree with a successful Launch and Doctor.
- Start each drive with a fresh evidence directory and its own scratch data.
- Stop capture processes before cleaning real data. This verification uses synthetic, immutable capture files.

## Driving conventions

Run `scripts/declarations.py drive` through the run's Python, as shown in the skill. One invocation exercises every feature listed below. The CLI is short-lived and noninteractive, so the helper records subprocess output directly. There is no shared server to drive.

## Proof and skip reporting

Pair each command and its exit code with the saved Parquet rows. `proof.json` exists only after every assertion passes. Do not claim fetch, import, malformed-input handling, or every possible spelling is covered by this fixture.

## Features

- [Funding roles](funding-roles.md) proves separate observations and correct joins, including identical names and repeated payments.
- [Name features](name-features.md) proves shared cleaning across roles and preservation of original names.
- [Capture preservation](capture-preservation.md) proves both output tables, unchanged captures, and rejection of an existing destination.
