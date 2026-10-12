# Exposed search verification map

Use the frozen `exposed_search_regression` database and an API started from the
current worktree as described in [verify](../SKILL.md). Run the HTTP check once
to cover all cases below. The default strictness is `0.8` for all regression searches,
with a limit of ten results per entity type.

- [Canonical search and alias matches](search.md) covers `unite`, `west mid`, and `hsbc`.
- [Recorded funder names](recorded-names.md) covers the exact Unite and HSBC profiles and aliases.

The populated funder grouping regression checks the documented BBC, GB News,
and Premier League results through `/search` and `/funders/{id}`. It requires an
isolated copy of the 12 October 2026 development data with the group migration
applied and `exposed data refresh-funder-groups` run. Follow the
[populated funder groups procedure](../SKILL.md#populated-funder-groups).

The test expects one result per search, the documented source and funding entry
counts, recorded names, and both kinds of direct link evidence.

The baseline was captured from the accepted development data on 12 October 2026.
IDs are arbitrary and do not affect comparisons. Group member IDs are replaced
with stable labels, and direct links use those same labels. Refreshing the snapshot can
change names. Review those data changes
separately from search-code changes before editing the accepted JSON.
The map covers HTTP behavior. Browser interactions and ingestion resolution
are outside this focused regression check.
