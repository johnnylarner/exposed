# Search

The [manifest](../live-cases.json) selects `unite`, `west mid`, `hsbc`,
`john mcdonnell`, `jahn`, `bbc`, `gb news`, and `premier league`.
The public search loader owns strictness 0.8 and the limit of ten per entity type.
Compare entity order and multiplicity, MP Parliament IDs, names, kinds,
funder context and name/alias match provenance. Organisation UUIDs are local to
each installation and remain in raw evidence only.

Run [the complete workflow](../SKILL.md). Selected searches must return nonempty successful results;
error states, empty results and missing result fields cannot match. Every selected detail identity
must independently resolve exactly once in each environment.

Equal-score funder matches sort by UUID. Different installation-local IDs can
change result order and the ten-result cutoff even though UUIDs are omitted
from semantic comparisons. Investigate snapshot differences before attributing
these failures to search code.
