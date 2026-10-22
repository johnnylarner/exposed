# Exposed live verification map

Run the snapshot-backed worktree comparison in [verify](../SKILL.md). The live
production loaders provide the comparison reference on every run. Selectors live
in [live-cases.json](../live-cases.json); none freeze monetary expectations.

- [Search](search.md): order, duplicates, identity context and match provenance.
- [MP totals](member-totals.md): John McDonnell and Alex Burghart.
- [Organisation totals](funder-totals.md): BBC, GB News, Premier League, HSBC and Unite.

This map covers selected public loader data. It does not verify rendered browser
interactions, ingestion resolution, or full declaration histories. Focused
application unit/integration tests remain separate checks. A historical snapshot
can legitimately differ from production; record those failures without altering
source data or claiming parity.
