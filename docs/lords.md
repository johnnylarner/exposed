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
`exposed.members` owns the shared UUID and Parliament identity. Commons and Lords
profiles are separate rows in `exposed.member_profiles`, keyed by `(member_id, chamber)`.
The database chamber enum and source-shape check enforce each profile's required fields.
Commons writes supply their creation and update clocks explicitly; Lords rows have
only their source capture clock. The migration preserves existing Commons clocks.
The Lords profile retains the full source `item.value` JSON document as evidence,
alongside typed profile fields, capture time, and import key. Ordered categories and
interest trees live in `lords_categories` and `lords_interests`. Positional composite
keys preserve repeated source IDs, identical interests, and empty categories.
One transaction writes the ledger, identities, profiles, and complete occurrence trees.
The profile and its tree change only when the incoming `(captured_at, ingestion_key)` pair is greater.
The UUID breaks ties between captures at the same database timestamp.
Imports lock member rows in Parliament ID order.
Members omitted from a later capture retain their latest stored document.

Both importers acquire the same identity in stable Parliament ID order. The Lords importer
never changes Commons profile fields, declarations, funders, or funding entries.
A Parliament ID can have both profiles. Commons cohorts require a Commons profile,
including former MPs. Generated chamber columns and composite foreign keys attach
declarations only to Commons profiles and categories only to Lords profiles.
Lords interests do not enter financial totals or rankings.

## HTTP and search

Detail reads reconstruct the ordered tree from relational rows in one repeatable-read
snapshot. Search reads the Lords profile. Neither public path parses the retained JSON.
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
