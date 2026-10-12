# Entity detail reads

Search entities carry stable string IDs. `MP` IDs are positive Parliament member
numbers. `Funder` IDs are database UUIDs for canonical funder identities. The HTTP
adapter parses these IDs before calling the detail service.

`GET /members/{id}` returns `member`, `declarations`, and `summary`
from one read-only repeatable-read transaction. `declarations`
contains every stored declaration. Registration date sorts descending, with
missing dates last. Source declaration ID breaks ties. Every funding occurrence
survives, including duplicates and entries with missing fields. Declarations
without funding entries remain visible. The declaration count is the length of
this complete array.

`summary` contains independent exact currency totals and category breakdowns over
all funding occurrences in `declarations`. It also reports funding entry counts,
unknown currency counts, first and last known registration dates, and the number
of undated declarations. Each currency and category total includes local counts
of unavailable amounts. A null total means no amount is known. Known zero and
negative amounts remain distinct from null.

The domain owns `DeclarationCategory`. Source category ID 3 means donations and
other support, including loans. Every other category retains its source ID and
label, including unrecognized IDs. The classification does not infer direct
personal receipt.

The frontend summary at `/members/{id}` shows a compact latest profile, known
values, the first five declarations from the complete ordered history with their
category labels, and category bars. Categories with negative
values share a central zero baseline. Monetary labels retain exact decimal
strings. Bar widths use bounded
integer ratios and do not sum money in the browser.

`/members/{id}/history` uses the same response. Inclusive registration date bounds
use UTC calendar days. The frontend filters the complete history before slicing
pages of ten. Changing or clearing dates resets the page. Undated declarations
appear last without bounds and are excluded when either bound is active. A start
date after the end date produces an error instead of results.

The profile's `membership_from` describes the latest membership, which can be a
Lords profile for former MPs.

`GET /funders/{id}` returns the canonical profile, occurrence counts, and one
summary per known currency. `funder.aliases` contains every unique stored name,
including the canonical name when stored, ordered by PostgreSQL's `C` collation.
Case, punctuation, and whitespace remain intact. An identity without stored
aliases returns an empty list. The UI shows nonempty lists as "Recorded names".
The repository returns a compact aggregate per recipient and currency in a
read-only repeatable-read transaction. The service uses those aggregates for both
party totals and recipient rankings.

Party groups use the recipient's latest stored party ID and name. They do not
represent direct payments to parties or party membership when support was
received. The service ranks up to ten recipients independently for each currency
by known amount descending, then Parliament member ID and name. Missing totals
sort after known totals.

Amounts use `BigDecimal` throughout Rust. HTTP response DTOs serialize normalized
plain decimal strings, without rounding. The browser groups digits using string
operations. Zero and negative values remain visible. Null or blank currencies
have no sum and are counted as excluded entries. Currency normalization removes
surrounding Unicode whitespace, including tabs, newlines, and the byte-order mark.
Funding entries expose this normalized currency, or null when it is unknown,
so history labels and summary coverage agree. Known currencies with missing
amounts retain local coverage counts and a null total when every amount is
missing. The service derives party totals before truncating recipient rankings.

Both routes return 404 for absent identities, 422 for malformed IDs, and a
sanitized 500 for unexpected repository failures. Existing identities without
declarations or funding remain successful empty responses.

The frontend uses ordinary links and selects the page from the pathname when the
document loads. Links retain `q` and `strictness` through search and detail pages.
The host must serve the frontend entry point for direct member and funder paths.
Vite supports this fallback. Detail requests time out after ten seconds and abort
when their component is destroyed. MP pages use purple declaration lists. Funder
pages use teal currency tables. Explicit text and icons distinguish both types.

SQLx fixture tests cover precision, duplicate occurrences, complete history,
older donations, category identity,
date ties, incomplete values, independent rankings, empty identities, and real
HTTP status handling. Controlled Playwright tests cover native navigation, reload,
new tabs, recovery, safe source text, and mobile layout. The project verification
map describes the separate live database and browser proof.

## Design choices

One recipient aggregate supplies both party totals and rankings. Separate SQL
queries would repeat grouping rules and could disagree between reads. Loading
every funding occurrence into the service would retain those rules but require
unbounded historical data in memory. The compact aggregate keeps exact sums and
local missing-value counts while limiting transfer to recipients and currencies.

Native document links keep browser history, reload, and new-tab behavior without
a client router or another navigation state. Member history is complete, while
recent declarations and funder recipient rankings retain their labelled limits.
