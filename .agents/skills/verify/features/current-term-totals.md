# Check current-term totals

Run these checks when verifying totals or rankings. Use the
[financial totals contract](../../../../AGENTS.md) and
[ReportingContext](../../../../exposed/src/lib/domain/models/reporting_period.rs)
as the scope. These are declared amounts, which can include rates and benefits.
Do not restrict the check to employment or earned income.

Require the following results:

- Include declarations registered at or after `2024-07-09T00:00:00Z`.
  Compare instants in UTC, including timestamps with an offset.
- Exclude earlier and undated declarations from current-term amounts and
  declaration counts. Retain those declarations in historical totals.
- Use registration dates, not payment dates, fetch dates, calendar-year starts,
  or the MP's membership start.
- For MPs, include every category, including unknown IDs. For the Individuals,
  Companies, and Parties ranking boards, retain the existing donation-support
  category scope. Apply the same term cutoff to every board.
- Preserve exact GBP decimal amounts, null versus zero, counts, and completeness.
  Missing GBP amounts and unknown currencies make totals incomplete. Known
  foreign currencies do not contribute to GBP amounts or make totals incomplete.

## Run the boundary and ranking tests

From the repository root, run the existing semantic tests:

```sh
cargo test -p exposed --lib funder_and_member_share_term_boundaries_and_matching_occurrence_totals
cargo test -p exposed --lib outbound::rankings::tests
```

The ranking tests require `DATABASE_URL` for a local disposable PostgreSQL test
server whose user can create databases. SQLx creates isolated fixture databases
and applies the repository migrations. Never point these tests at production or
the retained regression snapshot. A missing database prerequisite leaves the
current-term check unverified.

Inspect the [summary boundary test](../../../../exposed/src/lib/domain/services/entity_details.rs)
and [ranking assertions](../../../../exposed/src/lib/outbound/rankings/tests.rs)
alongside the [ranking fixture](../../../../exposed/src/lib/outbound/rankings/fixtures/rankings.sql).
Confirm that the tests execute and retain assertions for the cutoff cases.
The summary test includes an offset timestamp at the cutoff, an earlier record,
and an undated record. It checks current-term and historical amounts and counts
for both member and funder summaries.

The ranking fixture includes a declaration exactly at the cutoff, an earlier
declaration, and an undated declaration. The excluded declarations each contain
GBP amounts large enough to change the expected boards if included. Require the
fixture's exact totals and counts on all four boards, member-summary parity,
and the HTTP response's `current_term_start`.

## Report the evidence

Record the worktree commit, commands, exit codes, and test output. Report a failed
assertion as a failed current-term check. Report a command that cannot run or
matches no tests as unverified. Do not treat a matching production response as a
substitute for boundary evidence.

Run the [snapshot comparison](../SKILL.md) separately for live parity. Its current
manifest covers selected searches and detail summaries, not the rankings loader.
Do not claim live ranking parity from that command. Do not recompute page totals
in JavaScript or change production expectations to make either check pass.
