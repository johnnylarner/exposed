# Product marketing context

**Document version:** v1
**Last updated:** 2026-11-19

## Product overview

declair is a free, open-source tool for exploring UK parliamentary registers of interests. A reporter can look up an MP, see a summary of their declarations for the current parliamentary term, then read individual records. A reporter starting with an organisation can look it up as a funder and explore its links to MPs. Lords have separate registered-interest pages and histories.

## Target audience

Start with UK journalists covering Westminster, political funding and lobbying. The first job to design search copy around is briefing yourself on a named MP before an interview or while checking a story. The second is following a named organisation across MPs’ declarations. NGOs working on government accountability are also an intended audience in the [project description](../frontend/src/about.md). Their search needs still need research.

The November 2026 outreach shortlist identified investigative reporters including Henry Dyer, Ethan Shone and Peter Geoghegan through their coverage of political money and influence. That work informed prospect selection. The shortlist provides no evidence of product use or endorsement.

## Problems and switching dynamics

A journalist described wanting quick statistics and an overview. A plausible reporting workflow is to open a member profile for orientation, then check the declaration behind a figure before using it. A reader starting from a familiar parliamentary register already knows where the official record lives. A reason to try declair is that the period summary and individual declarations are available together. A reason to hesitate is uncertainty about what the summary includes.

## Competitive landscape and differentiation

UK Parliament’s registers are the primary source and the natural starting point for checking a declaration. Reporters can also compile records in a spreadsheet for a particular story. declair provides a reusable member summary and groups probable funder aliases. For example, a researcher looking for Google can encounter both “Google UK Ltd” and “Google UK Limited”. Showing the recorded names lets the reader inspect that grouping. The [methodology](../frontend/src/methodology.md) describes the sources and matching rules.

## Objections and scope

| Reader question | Answer |
| --- | --- |
| What does “current” mean? | For MP totals, the current term includes records registered from 9 July 2024 UTC. A registration date can differ from the date a benefit was received. Historical totals also include undated records. |
| Can I call the GBP total the MP’s earnings? | Use “declared GBP amount”. Declarations can include rates, loans, gifts and support for a party organisation. Known foreign currencies are excluded without conversion. Missing GBP amounts or unknown currencies can make a total incomplete. |
| Does a declaration count tell me how many payments there were? | A declaration is a registered record. It may contain zero, one or several funding entries. |
| Can I compare peers with MPs in the rankings? | Lords’ pages show registered interests and their history. The declared GBP rankings cover MP data. |

For a story about a party’s overall finances or a politician’s personal wealth, the parliamentary register answers a narrower question about registered interests.

## Customer language and brand voice

Jamie recalled one journalist asking for “getting quick statitics and overviews about an MPs current state” (approximate wording as supplied, 19 November 2026). Treat “current state” as a clue to the reader’s task. We still need to learn which figures they want first. This is interview evidence from one person, not a search-volume estimate or a publishable testimonial.

Use plain questions for discovery and precise labels beside figures. The existing search title, “Search MPs, Lords and funders”, names what a reader can look up. “Reading the data” introduces interpretation in everyday language. “Recorded names” gives the reader a concrete way to inspect a funder match. Use lowercase **declair**, UK spelling and the register’s terminology. Avoid “richest MPs”, “money pocketed” and “top donors” for rankings across all categories. Those phrases imply facts the figures cannot establish.

## SEO priorities and goals

| Priority | Reader intent to test | Page and proposed wording |
| --- | --- | --- |
| First | What has this MP declared this parliamentary term? | Member profile. Try “[MP name]’s registered financial interests” for the title. Show the reporting period alongside the summary. |
| Second | Which MPs have declared support from this organisation? | Funder profile. Try “[Organisation] in MPs’ financial declarations”. Keep the recorded names visible for verification. |
| General discovery | Where can I search UK parliamentary interests? | Homepage. Test “Search UK MPs’ and Lords’ registered interests &#124; declair” as the title. |

These are editorial hypotheses. The goal is for a journalist arriving from search to find the relevant profile and inspect a declaration. To evaluate the first priority, review organic queries reaching MP profiles for combinations of member names and interests terms, then check whether visitors inspect a record if that action can be measured. No organic traffic, conversion baseline or tested title result is established here.

## Proof points

Use the product itself as evidence. The [member profile](https://declair.work/members/5091) illustrates a named lookup. The [methodology](../frontend/src/methodology.md) documents a funder grouping and explains how readers can cite parliamentary data. The founder’s outreach describes the tool as free. No time-saved figure, newsroom adoption claim or testimonial has been established.

## Learnings

| Date | Learning | Evidence | Applies to | Status |
| --- | --- | --- | --- | --- |

## Changelog

- v1 (2026-11-19) — Initial context.
