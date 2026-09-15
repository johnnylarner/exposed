# UK Parliament APIs: members and registered interests

Verified against Parliament's OpenAPI documents and unauthenticated live GET requests on 15 September 2026. This note distinguishes documented behavior from observations and implementation recommendations.

## Which API to use

| Need | API | Notes |
| --- | --- | --- |
| Find MPs and peers; party, constituency, biography and membership history | [Members API](https://members-api.parliament.uk/index.html) | Base URL `https://members-api.parliament.uk`; paths use `/api/`, without a version segment. |
| Structured Commons financial interests, publications and exports | [Register of Interests API](https://interests-api.parliament.uk/index.html) | Prefer current `/api/v2/` for new work; `/api/v1/` remains available with different defaults and response structure. |
| Lords interests, or a simple member-by-member interest display | [Members API](https://members-api.parliament.uk/index.html) | Dedicated Lords register search and individual registered-interest endpoints; principally category and text data. |

Sources: [Members OpenAPI](https://members-api.parliament.uk/swagger/v1/swagger.json), [Interests v2 OpenAPI](https://interests-api.parliament.uk/swagger/v2/swagger.json), [Interests v1 OpenAPI](https://interests-api.parliament.uk/swagger/v1/swagger.json).

## Members API

Useful routes include:

| Route | Purpose |
| --- | --- |
| `GET /api/Members/Search` | Search members; use `House=1` for Commons or `House=2` for Lords and `IsCurrentMember=true` for current membership. |
| `GET /api/Members/{id}` | Retrieve a member by their numeric Parliament member ID; optional `detailsForDate` requests historical details. |
| `GET /api/Members/SearchHistorical` | Search at a date using `dateToSearchFor`. |
| `GET /api/Members/History?ids=...` | Member name, party and membership histories. |
| `GET /api/Members/{id}/Biography` | Parliamentary career information. |
| `GET /api/Members/{id}/Contact` | Published contact details. |
| `GET /api/Members/{id}/RegisteredInterests` | Categories containing the member's registered interests. |
| `GET /api/LordsInterests/Register` | Search the Lords register. |

Search results contain `items[].value`, plus `totalResults`, `skip` and `take`; a member lookup returns its data in `value`. Member fields include `id`, `nameDisplayAs`, `latestParty`, `latestHouseMembership` and `thumbnailUrl`. Commons membership contains constituency information and membership status/dates. Member search has a maximum `take` of 20. The Lords register instead uses a zero-based `page`, with 20 records per page, and supports `searchTerm` and `includeDeleted`. [Members OpenAPI](https://members-api.parliament.uk/swagger/v1/swagger.json)

The per-member registered-interests result contains categories and `interests[]` with an `interest` text field, `createdWhen`, `lastAmendedWhen`, `deletedWhen`, `isCorrection` and recursive `childInterests`. It is suitable for displaying a member's register; structured financial analysis is better served by the dedicated Interests API for Commons data. The preference for analytical work is an implementation recommendation based on the two response schemas. [Members OpenAPI](https://members-api.parliament.uk/swagger/v1/swagger.json), [Interests v2 OpenAPI](https://interests-api.parliament.uk/swagger/v2/swagger.json)

## Dedicated Register of Interests API

### Scope and endpoints

The v2 `RegisterType` enum currently contains `Commons` and `CommonsStaff`; it does **not** include Lords. Explicitly send `Type=Commons` for MPs' own interests. The documentation warns that omitting `Type` includes all types, including any added in future. Staff interests have a different registrant shape, so treating every result as an MP's own interest would be incorrect. [Interests v2 OpenAPI](https://interests-api.parliament.uk/swagger/v2/swagger.json)

| Route, under `https://interests-api.parliament.uk` | Purpose |
| --- | --- |
| `GET /api/v2/Interests` | Search published interests. |
| `GET /api/v2/Interests/{id}` | Retrieve an interest; documented as the latest published version. The v2 schema includes a `versions` array. |
| `GET /api/v2/Categories` | Get category IDs, names, numbers, parent categories and register type. |
| `GET /api/v2/Categories/{id}` | Retrieve one category. |
| `GET /api/v2/Registers` | Enumerate published registers, optionally filtered by `Type` and `SessionId`. |
| `GET /api/v2/Registers/{id}` | Retrieve publication metadata. |
| `GET /api/v2/Registers/{id}/document` | Download a PDF; `type=Full` is the default, `type=Updated` selects updates only. |
| `GET /api/v2/Interests/csv` | Download category CSVs packaged in a ZIP; `IncludeFieldDescriptions=true` adds category-field metadata. |

Endpoint contracts and parameters: [Interests v2 OpenAPI](https://interests-api.parliament.uk/swagger/v2/swagger.json).

Interest-search filters include `MemberId`, `CategoryId`, `RegisterId`, `InterestIds`, `Type`, `PublishedFrom/To`, `RegisteredFrom/To`, `UpdatedFrom/To`, `ExcludeExpired` and `ExpandChildInterests`. Dates are `YYYY-MM-DD`. Date boundaries are inclusive according to the descriptions. Sorting supports `PublishingDateDescending` and `CategoryAscending`. There is no documented free-text interest-search parameter. [Interests v2 OpenAPI](https://interests-api.parliament.uk/swagger/v2/swagger.json)

Live category discovery returned 12 Commons category records: ten main categories covering earnings, donations, gifts, overseas visits, property, shareholdings, miscellaneous interests and family-related disclosures, with additional 1.1 and 1.2 employment/earnings subcategories. Category database IDs differ from the displayed category numbers: for example, ID 3 is category number 2, donations. Fetch this reference data rather than deriving an ID from a printed number. [Live categories](https://interests-api.parliament.uk/api/v2/Categories?Type=Commons&Take=20)

### Response structure and joining members

The v2 list envelope is `{ skip, take, totalResults, items, links }`. Unlike member search, these `items` are direct interest objects, without a `value` wrapper. An interest contains:

- `id`, `parentInterestId`, `category` and `childInterests`.
- `registrant.type`, with `registrant.memberDetail` for MPs or `registrant.memberStaffDetail` for staff.
- `versions[]`, whose records contain `summary`, `registrationDate`, `publishedDate`, `updatedDates`, `rectified`, `rectifiedDetails`, `register` and `fields[]`.
- Links to related resources and pagination pages.

An MP's `registrant.memberDetail.id` is the ID used by the Members API. Live results include an explicit link such as `https://members-api.parliament.uk/api/Members/4736`. Use this numeric ID for joins. Names, party and House in the embedded member schema are described as **current** attributes, so they should not be assumed to describe the member at the time of an old declaration. [Interests v2 OpenAPI](https://interests-api.parliament.uk/swagger/v2/swagger.json), [Live interest example](https://interests-api.parliament.uk/api/v2/Interests/16903)

Fields are category-dependent records containing `name`, `description`, `type`, `typeInfo`, `value` and potentially nested `values`. In the live gift example, these include `DonorName`, `DonorStatus`, `Value`, `ReceivedDate`, `AcceptedDate` and donor company identifiers. The decimal amount is serialized as a string (`"1860.00"`) with `typeInfo.currencyCode="GBP"`; not every declaration supplies every field. Preserve category-specific fields and original text when normalizing. [Live interest example](https://interests-api.parliament.uk/api/v2/Interests/16903), [Interests v2 OpenAPI](https://interests-api.parliament.uk/swagger/v2/swagger.json)

`ExpandChildInterests=true` nests related interests; otherwise they appear as separate list items. `parentInterestId` describes a payment's association with its payer interest. Avoid counting both a parent arrangement and its child payments as independent sums without understanding their fields. This is a modeling recommendation based on the documented relationship. [Interests v2 OpenAPI](https://interests-api.parliament.uk/swagger/v2/swagger.json)

### Pagination, versions and historical data

- Lists use `Skip` and `Take`, defaulting to 0 and 20, and provide navigation links. v2 does not document a maximum `Take`; live requests for 100 and 101 both returned that many items. Do not carry over v1's documented maximum of 20 to v2. [v2 specification](https://interests-api.parliament.uk/swagger/v2/swagger.json), [v1 specification](https://interests-api.parliament.uk/swagger/v1/swagger.json), [live v2 page](https://interests-api.parliament.uk/api/v2/Interests?Type=Commons&Take=101)
- **v2 without `RegisterId` searches all registers, while `ExcludeExpired=true` is the default.** Set `ExcludeExpired=false` when intentionally retrieving expired interests and choose `RegisterId` for a specific publication. v1 without `RegisterId` instead defaults to the latest register. [v2 specification](https://interests-api.parliament.uk/swagger/v2/swagger.json), [v1 specification](https://interests-api.parliament.uk/swagger/v1/swagger.json)
- Registration, publication, updates and the underlying event/payment date are separate concepts. The event date can appear within category-specific fields. Date filtering is not interchangeable with selecting a published register. [v2 specification](https://interests-api.parliament.uk/swagger/v2/swagger.json), [live interest example](https://interests-api.parliament.uk/api/v2/Interests/16903)
- The public publication list sends users to the older Parliament publications site for editions before 2024. The live API nevertheless returned register metadata from 18 March 2024 onward, so the September 2024 public API launch must not be treated as a strict data start date. Metadata availability alone does not establish structured-interest completeness for older editions. [Publication archive](https://members.parliament.uk/members/commons/interests/publications), [live register list](https://interests-api.parliament.uk/api/v2/Registers?Type=Commons&Take=100), [Parliament's launch report](https://pds.blog.parliament.uk/2024/11/27/working-towards-a-fully-searchable-register-of-members-financial-interests/)
- MPs must register relevant changes within 28 days; Parliament says interests remain on the Register for 12 months after expiry. A current register is therefore not a lifetime financial record. The registration rules do not by themselves define the precise behavior of every API expiration filter. [Official register overview](https://www.parliament.uk/mps-lords-and-offices/standards-and-financial-interests/parliamentary-commissioner-for-standards/registers-of-interests/register-of-members-financial-interests/)

For reproducible ingestion, record the API version, query, retrieval date and publication IDs; retain raw responses and upsert by interest ID while preserving versions and child relationships. For periodic syncing, `UpdatedFrom` is useful but the schema does not describe a complete deletion/change-feed protocol. Those are implementation recommendations, not service guarantees. [Interests v2 OpenAPI](https://interests-api.parliament.uk/swagger/v2/swagger.json)

### Minimal examples

```sh
# First page of current Commons members.
curl 'https://members-api.parliament.uk/api/Members/Search?House=1&IsCurrentMember=true&skip=0&take=20'

# Published Commons interests for one member, including expired records.
curl 'https://interests-api.parliament.uk/api/v2/Interests?Type=Commons&MemberId=4736&ExcludeExpired=false&ExpandChildInterests=true&Skip=0&Take=20'

# Publication IDs to use when selecting a register.
curl 'https://interests-api.parliament.uk/api/v2/Registers?Type=Commons&Skip=0&Take=20'

# Interest data for a specific publication; replace 820 as appropriate.
curl 'https://interests-api.parliament.uk/api/v2/Interests?Type=Commons&RegisterId=820&ExcludeExpired=false&Skip=0&Take=20'
```

These URL forms were checked with unauthenticated live GETs; publication 820 was the Commons register dated 7 September 2026 at verification time. [Live publication](https://interests-api.parliament.uk/api/v2/Registers/820)

## Access, operations and reuse

Both APIs answered the sampled GET requests without authentication, and neither inspected OpenAPI document declares a security scheme. No explicit requests-per-second allowance was found in these specifications; this should not be described as unlimited access. Sampled member responses advertised `Cache-Control: public,max-age=300`. Caching, moderate concurrency and retry/backoff handling are reasonable client choices. [Members OpenAPI](https://members-api.parliament.uk/swagger/v1/swagger.json), [Interests v2 OpenAPI](https://interests-api.parliament.uk/swagger/v2/swagger.json)

Parliament explicitly makes register data available under the Open Parliament Licence. Its licence permits commercial and non-commercial reuse subject to its conditions and exclusions. Required attribution: “Contains Parliamentary information licensed under the Open Parliament Licence v3.0.” Link to the licence where possible. [Register overview](https://www.parliament.uk/mps-lords-and-offices/standards-and-financial-interests/parliamentary-commissioner-for-standards/registers-of-interests/register-of-members-financial-interests/), [Open Parliament Licence](https://www.parliament.uk/site-information/copyright-parliament/open-parliament-licence/)

## Suggested workflow: current MPs and their available disclosure history

Interpret current members as today's Commons officeholders. Page through:

```text
https://members-api.parliament.uk/api/Members/Search?House=1&IsCurrentMember=true&skip=0&take=20
```

Collect `items[].value.id`, increasing `skip` until all results are retrieved. Avoid `MembershipStartedSince`: a long-serving current MP's membership may start years before this Parliament. Add `IsEligible=true` if the intended scope specifically requires current eligibility to sit. [Members schema](https://members-api.parliament.uk/swagger/v1/swagger.json)

Then page through:

```text
https://interests-api.parliament.uk/api/v2/Interests?Type=Commons&ExcludeExpired=false&ExpandChildInterests=false&Skip=0&Take=100
```

Retain interests whose `registrant.memberDetail.id` belongs to the current-MP ID set. Follow `nextPage` links, or advance the offset by the number of API items received before local filtering. Omitting `RegisterId` searches all registers; omitting all date filters avoids cutting off disclosures before this Parliament. Setting `ExcludeExpired=false` includes expired interests. With `ExpandChildInterests=false`, retain `parentInterestId` to reconstruct relationships and preserve every returned `versions[]` record. These are implementation recommendations based on the [v2 schema](https://interests-api.parliament.uk/swagger/v2/swagger.json).

Alternatively add `MemberId={id}` and paginate separately for each current MP. For a full initial import, the bulk approach provides a single interest pagination stream followed by a local membership join.

An unfiltered live member-172 query returned 15 interests, including a June 2024 publication and a record registered in October 2016. This verifies that the call can include older disclosures; it does not establish a complete lifetime history. Parliament directs users to older publication archives for editions before 2024, so comprehensive earlier coverage requires separate archival work. [Verified query](https://interests-api.parliament.uk/api/v2/Interests?Type=Commons&MemberId=172&ExcludeExpired=false&ExpandChildInterests=false&Take=20), [Publication archive](https://members.parliament.uk/members/commons/interests/publications)

## First application milestone: everyone who served in this Parliament

The agreed first build extends member coverage beyond today's cohort: include everyone who has served in the Commons since this Parliament's election, including people who have already left. Interests ingestion remains a later milestone. The term stores `term_start = 2024-07-04` and nullable `term_end`; members' service dates remain separate. Implementation and operational details are in the [project README](../../ingest/README.md).

Page through the historical cohort, fixing the upper date at the start of each run:

```text
https://members-api.parliament.uk/api/Members/Search?MembershipInDateRange.WasMemberOnOrAfter=2024-07-04&MembershipInDateRange.WasMemberOnOrBefore=2026-09-15&MembershipInDateRange.WasMemberOfHouse=1&skip=0&take=20
```

Omit the top-level `House`, `IsCurrentMember` and `IsEligible` filters on this query. In particular, top-level `House` means the member's **most recent** House and could exclude a former MP who later joined the Lords. There is no documented sort parameter; “sort by newest” is not an ingestion strategy. [Members OpenAPI](https://members-api.parliament.uk/swagger/v1/swagger.json)

Fetch the current Commons query separately for current status, and batch the cohort's IDs through `/api/Members/History?ids=172&ids=4736`. Repeated `ids` parameters were verified live. Validate the cohort against actual Commons service intervals from `houseMembershipHistory`; the specification does not fully explain date-range matching across disjoint membership periods. [Verified history batch](https://members-api.parliament.uk/api/Members/History?ids=172&ids=4736)

The latest profile for member 172 reports a continuous start of 11 June 1987, while the history response contains a current Commons period starting 4 July 2024 and a preceding period ending 30 May 2024. Use history for dated term service and set the term-specific start to the later of the source start and election day. Older history records do not all use identical end-date conventions, so a future closed-Parliament backfill needs additional validation. [Member profile](https://members-api.parliament.uk/api/Members/172), [Member history](https://members-api.parliament.uk/api/Members/History?ids=172)

The initial sampled historical search advertised 655 members and the current Commons search 649 on 15 September 2026. These are observed counts, not invariants to encode in tests. [Historical query](https://members-api.parliament.uk/api/Members/Search?MembershipInDateRange.WasMemberOnOrAfter=2024-07-04&MembershipInDateRange.WasMemberOnOrBefore=2026-09-15&MembershipInDateRange.WasMemberOfHouse=1&skip=0&take=20), [Current query](https://members-api.parliament.uk/api/Members/Search?House=1&IsCurrentMember=true&skip=0&take=20)
