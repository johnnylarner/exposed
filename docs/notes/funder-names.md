# Funder names: investigate later

Status: deferred. Recorded from the local data audit on 17 September 2026; source
names remain unchanged.

There were 62 groups of funder names that become equal after trimming surrounding
whitespace, collapsing repeated whitespace, and lowercasing. These are candidate
text matches, not confirmed shared donor identities.

| Examples | Funding rows across variants |
| --- | ---: |
| `UNITE The Union`, `UNITE the Union`, `Unite The Union`, `Unite the Union` | 87 |
| `UNISON`, `Unison` | 85 |
| `National Liberal  Club`, `National Liberal Club` | 59 |
| `Confidential`, `confidential` | 40 |

The last example is a disclosure placeholder, not evidence of one common funder.
Case or spacing normalization also cannot establish that two people or companies
with the same name are the same entity.

Questions for the later investigation:

1. Would a normalized search/grouping key be sufficient while retaining the source
   name for display and traceability?
2. Which aliases need explicit review before they can share a funder identity?
3. How should placeholder names be excluded from identity matching?

Read-only query to reproduce the candidate groups:

```sql
SELECT lower(regexp_replace(trim(funder), '\s+', ' ', 'g')) AS normalized_name,
       array_agg(DISTINCT funder) AS source_names,
       count(*) AS funding_rows
FROM exposed.funding_entries
WHERE funder IS NOT NULL
GROUP BY 1
HAVING count(DISTINCT funder) > 1
ORDER BY funding_rows DESC;
```

The current [funding model](../../ingest/src/exposed/core/declarations.py) stores
only the selected source name. Canonical donor identities and alias matching
remain outside the ingestion specification until this investigation is complete.
