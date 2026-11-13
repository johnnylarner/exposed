# Lords registered interests

`exposed lords` imports the full Lords register from the Parliament Members API.
The source request is `GET /api/LordsInterests/Register?page=1&includeDeleted=true`.
Subsequent requests increment `page`. The source treats page zero as page one.

## CLI

Both stages use a YAML configuration containing `data_dir`.

```yaml
data_dir: ./data
```

```sh
exposed lords fetch --config lords.yaml
exposed lords load --config lords.yaml --ingestion-key <capture-uuid>
```

Fetch accepts an optional `--ingestion-key` and otherwise creates a UUID-v7.
Its response contains `ingestion_key`, `path`, `members`, and `pages`.
Load requires the captured key and the `DATABASE_URL` environment variable.
Its response contains `ingestion_key`, `members`, and `outcome`.
The outcome is `loaded` or `already_loaded`.

Both stages accept `--schema` without configuration, network, files, or a database.
Success produces one JSON document on stdout with exit code zero.
Execution failures produce `{"error":{"message":"..."}}` on stderr with exit code one.

## Capture and persistence

The artifact directory is `<data_dir>/<ingestion-key>/raw/lords`.
It contains the exact response bytes in `page-1.json`, `page-2.json`, and subsequent pages.
`manifest.json` records the capture time, source, member count, page lengths, and SHA-256 digests.

Fetch verifies contiguous offsets, stable totals, full intermediate pages, forward progress,
and unique member identities. Only a complete capture is published, through an atomic rename
that rejects an existing destination. A failed fetch leaves no published Lords artifact.
Load verifies the manifest and every page before opening its database transaction.

`exposed.lords_imports` records each committed capture and its digest.
An exact retry returns `already_loaded`. Reusing a key with different evidence fails.
`exposed.members` is the single canonical profile for each Parliament identity and UUID.
Both sources update its name, nullable party ID/name, membership description, kind,
source start/end timestamps, and status. Membership kinds reference the chambers lookup.
The common Rust membership type interprets Lords subtype IDs within that chamber only;
unknown subtypes retain their original description under `other_lords`.

Source activity and membership end dates are independent. Leave of absence, suspension,
and other inactive statuses do not establish that membership ended. The public Commons
flag is derived from an open Commons membership, and is null when the latest chamber
is unknown. Migrated Commons flags retain their
known current/former classification through explicit legacy status provenance; unknown
source dates and capture clocks remain null. Migration updates preserve original clocks.
No end date or Parliament capture timestamp is manufactured by the backfill.

Both importers lock members in Parliament ID order and use the same canonical freshness
rule. A known source `(captured_at, capture_key)` supersedes an unknown clock; known
clocks compare lexicographically, using the UUID to resolve equal timestamps. Legacy
Commons artifacts can establish a profile when no source clock is known, but cannot
replace a timestamped profile. Enriched Commons Parquet artifacts retain all shared
membership fields and their capture clock/key. Replays retain their original ordering.

`commons_imported_at` retains the imported MP cohort, including former MPs and members
with no declarations. A later Lords membership changes the shared profile shown on both
views. It never removes the member's Commons declarations, funding or search identity.
Latest party grouping follows this shared profile, not party affiliation at registration.
Missing party identity remains null; financial summaries retain an unknown-party bucket.
Amounts, currencies, registration periods, occurrence counts and completeness rules are
unchanged. Plain simple declarations never enter funding totals or rankings.

`simple_declaration_registers` stores only the register capture clock/key, raw source
`item.value`, and an outline of empty categories. It contains no copied profile fields.
Its freshness is independent of the canonical member clock, so a newer profile refresh
does not suppress a valid older register capture. `simple_declarations` stores each real
source occurrence, including deleted entries and nested children. Category source ID,
name, sort order and occurrence position are inline on each declaration. Positional keys
preserve repeated IDs and identical text. Empty categories have no synthetic declaration.

One transaction writes the ledger, member updates, register header and occurrence tree.
An exact ledger retry returns `already_loaded`, while conflicting reuse of a capture key
fails even after a newer capture has replaced its records. The ledger also records empty
captures and captures that lose every freshness comparison. Members omitted from a later
capture retain their latest stored register. A failure rolls back every part of the load.

The down migration requires every retained Commons member to fit the old schema.
It refuses unknown chamber, party ID/name or membership description, and refuses to
remove a member outside the imported Commons cohort who owns financial declarations.
The failure occurs before destructive statements in SQLx's migration transaction, leaving
the schema, records and capture ledger intact. It never invents party zero, empty source
labels or a Commons chamber to force a downgrade. A successful downgrade removes the
Lords-only register data and members, while retaining representable imported MP records.

## HTTP and search

Detail reads reconstruct the ordered tree from relational rows in one repeatable-read
snapshot. Search reads the canonical member profile for imported Lords registers. Neither public path parses the retained JSON.
Invalid stored positions or source values produce a database error rather than a partial tree.

`GET /lords/{id}` returns `member`, `captured_at`, and `categories`, without a wrapper.
The member ID is a positive Parliament identity serialized as a string.
The member includes the source display name and nullable party, membership description,
membership status, and membership end date.

Every category occurrence retains its `source_id`, nullable `name`, `sort_order`,
`placement`, and `interests`. Shareholding IDs 4, 12, 31, 1002, 1003, 1004, and 1005
have placement `shareholdings`. Every other ID has placement `other`, including Nil
and unknown categories. The backend owns this classification.

Each interest retains `source_id`, nullable `text`, nullable `created_when`,
nullable `last_amended_when`, nullable `deleted_when`, `is_correction`, and `children`.
Category and interest occurrences remain in source order, including repeated IDs and text.
Children retain the same recursive shape. Nullable source collections become empty arrays.
The raw document still preserves those source nulls.
Only `deleted_when` identifies deletion. A deleted parent does not mark its children deleted.
The API preserves text and validated lifecycle timestamp strings verbatim.
Offset-free source times do not acquire an inferred timezone.
`captured_at` is UTC capture provenance.

The frontend routes `/lords/{id}` and `/lords/{id}/history` use the same API response.
There is no reporting-period filter or separate history API endpoint for Lords.
The summary omits occurrences with their own deletion date, promoting surviving descendants past deleted ancestors without changing source order.
It derives plain shareholding labels from supported source syntax and otherwise shows a short source preview.
Other interests retain their full source text.
Interest history preserves every supplied category occurrence and complete root subtree, including repeated IDs and text.
It shows ten root cards per page, with empty source categories retained as empty cards.
Each item shows its full source text, source ID, available lifecycle timestamps verbatim, and its own deletion and correction status.
The capture timestamp is provenance, not a registration date.
History represents the latest captured register, not a sequence of previous text versions.
Absent profiles return 404, malformed IDs return 422, and unexpected failures return a sanitized 500.
Search returns `kind: "Lords"`, the string Parliament ID, `funder_kind: null`,
and `match_source: {"kind":"name"}`. Lords primary name matches join the existing
score ordering. They do not change MP or funder phonetic fallback eligibility.
