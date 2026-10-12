# Exposed search verification map

Use the frozen `exposed_search_regression` database and an API started from the
current worktree as described in [verify](../SKILL.md). Run the HTTP check once
to cover all cases below. Each case has an explicit threshold and result limit.

- [Canonical search and alias matches](search.md) covers `unite`, `west mid`, and `hsbc`.
- [Recorded funder names](recorded-names.md) covers the exact Unite and HSBC profiles and aliases.

The baseline was captured from the accepted development data on 12 October 2026.
IDs are arbitrary and do not affect comparisons. Refreshing the snapshot can
change names. Review those data changes
separately from search-code changes before editing the accepted JSON.
The map covers HTTP behavior. Browser interactions and ingestion resolution
are outside this focused regression check.
