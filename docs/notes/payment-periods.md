# Payment periods: investigate later

Status: deferred. Recorded from the data audit on 17 September 2026; no parser or
schema changes are proposed for the current task.

The local database contained 223 ongoing-payment declarations. Fetching those
source IDs from Parliament and selecting their latest register versions showed:

| `RegularityOfPayment` | Declarations |
| --- | ---: |
| Monthly | 160 |
| Yearly | 55 |
| Quarterly | 8 |

The importer stores `Value` as the funding amount but discards
`RegularityOfPayment`. Those amounts therefore measure different periods even
though all current funding rows use GBP. Summing them does not produce a
comparable earnings total.

Source examples:

- [7639](https://interests-api.parliament.uk/api/v2/Interests/7639): monthly, £3,126.23.
- [6170](https://interests-api.parliament.uk/api/v2/Interests/6170): yearly, £12,469.
- [7325](https://interests-api.parliament.uk/api/v2/Interests/7325): quarterly, £7,142.50.
- [14453](https://interests-api.parliament.uk/api/v2/Interests/14453): marked yearly,
  but the description says the amount covers the first six months of a contract.
  The source period label alone may not support automatic annualization.

Questions for the later investigation:

1. Does `Value` consistently mean a payment per stated period, or can it represent
   a cumulative or contract amount?
2. Should the source period be retained as a nullable field on each funding entry?
3. If comparable estimates are needed, what evidence and assumptions should an
   annualized value require, and when should it remain unavailable?

Keep source amounts intact while these questions are unresolved. The relevant
code is [funding extraction](../../ingest/src/exposed/adapters/declaration_models.py)
and [the funding model](../../ingest/src/exposed/core/declarations.py).
