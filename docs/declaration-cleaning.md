# Declaration cleaning reference

`data --config CONFIG declarations clean --ingestion-key UUID` reads an existing raw
declaration dataset. The YAML configuration needs `data_dir`. The command needs no
database connection or network access at runtime.

The command writes two Parquet files under
`<data_dir>/<UUID>/cleaned/declarations/`. Both files close before the command
renames their staging directory into place. An existing output causes an error.
A failed read or projection check publishes neither table. An interrupted write
can leave a hidden staging directory under `cleaned`; another invocation uses a
different staging directory. Raw evidence remains unchanged.

Raw fetch has no completion marker. Successful cleaning describes the member
partitions present when the command reads them. It does not establish that the
full intended cohort was fetched. Run it after acquisition has stopped.

## Funding occurrences

`funding_entries.parquet` has one row per actual funding occurrence in the latest
published version. It retains identical repeated payments as different rows.
The complete retained `source_json` distinguishes a declaration with no funding
from a genuine all-null funding occurrence. The cleaner checks declaration
metadata and the funding-value multiset against the raw Parquet projection.

Each row retains member and Parliament member IDs, declaration and parent IDs,
category, selected register, dates, original funding names, donor kind, company
number, amount, currency, payment type, and the ultimate-payer flag. Amounts and
company numbers remain source strings.

`funding_entry_id` is `<member UUID>/<declaration ID>/<register ID>/funding/<ordinal>`.
`funding_ordinal` is zero-based order in the replayed source. IDs are independent
of cleaned names and repeat across identical captures. A changed source order
changes the corresponding occurrence identities.

The nullable `donor_funder_id`, `payer_funder_id`, and `ultimate_payer_funder_id`
reference the corresponding observations in `funders.parquet`. An absent role
has no reference. Donor metadata can create a donor observation without a name.
An all-null funding occurrence has no funder references.

## Funder observations

`funders.parquet` represents source-role observations, not resolved people or
companies. Each row includes member, declaration, and register IDs, a role, a
scope, and a JSON pointer into the declaration's retained raw source.

| Scope | Identity | Funding reference |
| --- | --- | --- |
| `funding_entry` | `<funding_entry_id>/<role>` | Non-null funding ID and ordinal. |
| `declaration` | `<member UUID>/<declaration ID>/<register ID>/declaration/<role>` | Null funding ID and ordinal. |

Declaration observations retain latest top-level names and donor metadata when
those fields have no `Value` or `PaymentType` funding anchor. They can coexist
with nested funding entries. A top-level funding occurrence supplies its own
observations without an additional declaration observation.

Roles are `donor`, `payer`, and `ultimate_payer`. Only donor observations receive
`donor_kind` and `donor_company_number`. The cleaner does not select a preferred
role, propagate parent names to child declarations, or reconcile company-number
conflicts. Two matching names still have distinct observation IDs, including
confidential names.

## Name features

All features are independent candidates for future resolution. No feature is an
entity ID or a reason to merge rows.

| Column | Rule |
| --- | --- |
| `name_raw` | Original string, including punctuation, whitespace, and placeholders. |
| `name_status` | `missing` for absent or whitespace-only names; `withheld` for the exact normalized placeholders listed below; otherwise `present`. |
| `name_normalized` | Unicode NFKC, Unicode lowercase, and trimmed, collapsed whitespace. Includes parenthetical and geographic text. |
| `name_accent_folded` | NFD decomposition of the normalized name with combining marks removed. No transliteration. |
| `name_tokens` | Normalized full name with ampersands converted to `and`, apostrophes joined, single-letter dotted acronyms joined, and other punctuation converted to token boundaries. |
| `name_token_key` | Lexically sorted tokens, including repeated tokens. |
| `primary_name_normalized` | Normalized name excluding explicit trading-as text and parenthetical trading-as annotations. Other parentheses remain. |
| `organisation_core` | Primary-name tokens with one complete trailing UK legal suffix removed. |
| `person_core` | Full-name tokens with listed leading honorifics removed. |
| `person_initials` | First character of each person-core token. |
| `organisation_initials` | First character of each organisation-core token, excluding listed connective words. |
| `parenthetical_text` | Original contents of outer parentheses, in source order. An unmatched opening retains the remaining text. |
| `explicit_aliases` | Original names following a literal `trading as` or `t/a` marker. Parenthetical markers do not also create whole-string aliases. |
| `alias_normalized` | NFKC, lowercase, and whitespace-normalized forms of the explicit aliases. |
| `explicit_acronym` | An uppercase source acronym of 2 to 10 letters, allowing dots and whitespace, whose lowercase letters match the connective-free initials of its parenthetical expansion. Expansion before the acronym is also supported. |
| `acronym_expansion_normalized` | Normalized full expansion of a validated explicit acronym. |
| `has_conjunction` | A literal `and` token or ampersand. Does not assert multiple entities. |
| `has_unbalanced_parentheses` | An unmatched opening or closing parenthesis. |

Exact withheld placeholders are `confidential`, `withheld`, `name withheld`,
`anonymous`, and `not disclosed`. Missing and withheld names retain `name_raw` and
their status but have no primary matching features.

Legal suffixes are `limited`, `ltd`, `plc`, `llp`, `lp`, `cic`, `public limited
company`, `community interest company`, `limited liability partnership`, and
`limited partnership`. Removal requires a whole trailing token sequence. Leading
honorifics are `mr`, `mrs`, `ms`, `miss`, `mx`, `dr`, `prof`, `professor`, `sir`,
`dame`, `lord`, `lady`, `rev`, `reverend`, `rt`, and `hon`. Connective words for
organisation initials and acronym checks are `a`, `an`, `and`, `for`, `in`, `of`,
`on`, `the`, and `to`.

| Original name | Selected features |
| --- | --- |
| `Labour Together Limited` and `LABOUR TOGETHER LTD.` | Both organisation cores are `labour together`. Observation IDs remain distinct. |
| `National  Liberal Club` | Normalized name is `national liberal club`. |
| `James O’Brien` | Tokens are `james obrien`. Raw and normalized names retain the apostrophe. |
| `Friends of Sinn Féin Canada` | Accent-folded name is `friends of sinn fein canada`. Full-name features retain `canada`. |
| `CSC Computer Sciences Limited (Trading as DXC Technology Ltd)` | Primary organisation core is `csc computer sciences`; explicit alias is `DXC Technology Ltd`. Full-name tokens keep the annotation. |
| `ASLEF (Associated Society of Locomotive Engineers and Firemen)` | Explicit acronym is `aslef` with the full normalized expansion. |
| `BJD (GB) Limited` | Organisation core is `bjd gb`. The cleaner does not drop `GB` or infer it as an acronym. |
| `Alan Milburn & Ruth Briel` | Conjunction indicator is true. The cleaner does not split people. |
| `MPM Connect Ltd (a company controlled by Peter Hearn)` | Parenthetical evidence remains. There is no inferred alias for Peter Hearn. |

There is no fuzzy matching, phonetic encoding, external enrichment, or inferred
entity-type selection. The original donor kind remains available beside the
parallel person and organisation features.

## Public address evidence

The cleaner adds supplemental public-address evidence from checked source scopes.
Root `DonorPublicAddress`, `PayerPublicAddress`, and `UltimatePayerAddress` belong
to their respective roles. Nested `Donors` groups use `PublicAddress` only for
that nested donor. Addresses never propagate between root and nested scopes.

`address_raw` preserves absent and blank values. `address_normalized` uses NFKC,
lowercase, and collapsed whitespace. Explicit private, withheld, confidential,
and not-provided placeholders have no usable normalized value.
`address_source_field` records the source field name. `address_match_quality`
records `unavailable`, `partial`, or `numbered_street`. The last requires a numeric
house or building token and a street designation. The raw Parquet projection is
unchanged, so old captures can be replayed by this cleaner.

The [resolver](funder-resolution-design.md) requires this cleaned schema. Older
cleaned results remain immutable and require a fresh clean run from retained raw
capture before resolution.
