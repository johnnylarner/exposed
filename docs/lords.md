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
Its response contains `ingestion_key` and `members`.

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

`exposed.members` owns the shared Parliament identity and stable UUID. Both imports
upsert the latest name, required party ID/name and membership description, membership
kind, and optional source start/end timestamps and status. Numeric membership kinds
reference a non-null chamber. The canonical Rust enum classifies Lords source subtype
IDs within their chamber. Unsupported membership kinds fail the entire load.
Missing required source fields fail the entire capture at the source boundary.

The migration assumes the existing dataset contains only MPs and assigns every existing
member the elected MP kind. It preserves required party and membership fields, member
UUIDs, modification times and financial records. An unended Commons membership determines
the public current Commons flag. Source status text does not determine an end date.
Existing five-column Members Parquet artifacts remain readable. New artifacts retain the
membership kind and optional dates/status alongside those original fields.

`simple_declarations` references members directly. Each real occurrence retains inline
category ID/name/sort order and source category position, node/parent/sibling positions,
source ID, text, lifecycle timestamps, correction flag and fetch time. Deleted entries and
nested children remain present. Positions preserve repeated IDs and identical values.
Empty source categories produce no declaration and are omitted from database responses.
Raw source files remain in the artifact directory, outside the database.

One transaction upserts every captured member and replaces all their simple declarations.
Every successful repeat performs the same full load. Omitted members retain their stored
history. A failure rolls back every member change, deletion and insertion in the capture.
The down migration removes the new declarations and lookup tables, projects membership
kind/end dates back to the original house/current fields, and retains member records.

MP search and detail include current Commons members or members with financial declarations.
Financial queries continue to use the original declarations independently of latest chamber.
Shared latest party grouping, amounts, currencies, registration periods, declaration counts
and completeness rules are unchanged. Simple declarations never enter funding totals or
rankings.

## HTTP and search

Detail reads reconstruct the ordered tree from relational rows in one repeatable-read
snapshot. Search reads the canonical member profile for members with a Lords membership. Neither public path reads raw source JSON.
Invalid stored positions or source values produce a database error rather than a partial tree.

`GET /lords/{id}` returns `member` and `categories`, without a wrapper.
The member ID is a positive Parliament identity serialized as a string.
The member includes the source display name, required party and membership description,
and optional membership status and end date.

Every stored category occurrence retains its `source_id`, nullable `name`, `sort_order`,
`placement`, and `interests`. Shareholding IDs 4, 12, 31, 1002, 1003, 1004, and 1005
have placement `shareholdings`. Every other ID has placement `other`, including Nil
and unknown categories. The backend owns this classification.

Each interest retains `source_id`, nullable `text`, nullable `created_when`,
nullable `last_amended_when`, nullable `deleted_when`, `is_correction`, and `children`.
Category and interest occurrences remain in source order, including repeated IDs and text.
Children retain the same recursive shape. Nullable source collections become empty arrays.
The raw capture files preserve those source nulls.
Only `deleted_when` identifies deletion. A deleted parent does not mark its children deleted.
The API preserves text and validated lifecycle timestamp strings verbatim.
Offset-free source times do not acquire an inferred timezone.

The frontend routes `/lords/{id}` and `/lords/{id}/history` use the same API response.
There is no reporting-period filter or separate history API endpoint for Lords.
The summary omits occurrences with their own deletion date, promoting surviving descendants past deleted ancestors without changing source order.
It derives plain shareholding labels from supported source syntax and otherwise shows a short source preview.
Other interests retain their full source text.
Interest history preserves every supplied declaration occurrence and complete root subtree, including repeated IDs and text.
It shows ten root cards per page.
Each item shows its full source text, source ID, available lifecycle timestamps verbatim, and its own deletion and correction status.
History represents the latest captured register, not a sequence of previous text versions.
Absent profiles return 404, malformed IDs return 422, and unexpected failures return a sanitized 500.
Search returns `kind: "Lords"`, the string Parliament ID, `funder_kind: null`,
and `match_source: {"kind":"name"}`. Lords primary name matches join the existing
score ordering. They do not change MP or funder phonetic fallback eligibility.

## Refresh personal names for MPs and Lords

After importing member profiles, refresh personal names from Parliament MNIS preferred names.
Both chambers use the same commands and the same `data_dir` configuration.
Both stages require `DATABASE_URL` because fetch checks coverage of every stored member.

```sh
exposed data member-names fetch --config lords.yaml
exposed data member-names load --config lords.yaml --ingestion-key <capture-uuid>
```

Fetch captures `House=Commons/PreferredNames/` and `House=Lords/PreferredNames/`
from the official MNIS service. It fetches stored historical members individually when
absent from those responses. Each member must have exactly one explicitly open preferred
name with a nonempty `Forename` and `Surname`. Personal names retain compound parts.
Names are never inferred from titles or extracted from declaration text.

The immutable artifact `<data_dir>/<ingestion-key>/raw/member-names.json` contains a
versioned envelope with the original UTF-8 source documents, including their BOM.
A failed source request or incomplete response leaves no published artifact.
Fetch accepts an optional ingestion key. Load requires the captured key and replays offline.
Both commands return `ingestion_key`, `path`, and the number of stored `members` covered.
Both accept `--schema` without configuration or side effects.

Load parses the entire capture and checks every stored member's identity and chamber
before updating names in one transaction. It updates existing members only. The primary
`name` becomes the preferred first name and surname. `parliamentary_name` retains the
source `DisplayAs`, and `formal_name` retains `FullTitle`. Both remain searchable. Search
results and profile headings show the personal name with a distinct parliamentary name
secondarily. Other lists use the personal name. Replaying older Commons or Lords imports
preserves personal names and refreshes their source parliamentary spelling.

The migration preserves existing display names until the first refresh. Reverting it
restores parliamentary display names. Neither refresh nor migration changes membership,
declarations, funding, currencies, or financial totals.
