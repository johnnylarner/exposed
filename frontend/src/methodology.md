<header class="article-intro">
<h1 id="methodology-heading">Methodology</h1>

Exposed makes MPs' registered interests easier to search and explore. To build trust in our data we explain below where it comes from and how its processed.

</header>

<section aria-labelledby="interpretation">
<h2 id="interpretation">Reading the data</h2>

Exposed provides all declarations for sitting MPs. Until now all declarations have been made in GBP, so currency conversion has not been necessary. Some declarations are not monetary but are still shown here.

We are a small team and while we do our best to test our data we may miss some things!

</section>

<section aria-labelledby="sources">
<h2 id="sources">How we source and update our data</h2>

Exposed uses from the UK Parliament's [Members API](https://members-api.parliament.uk/index.html) and [Interests API](https://interests-api.parliament.uk/index.html). For every sitting member in parliament we pull all declarations associated with them via the parliament member ID. Our data is updated daily which means new declarations will appear on the site within 24 hours.

The Parliament API designates declarations older than 12 months as 'expired'. We don't acknowledge this distinction in our datal as ignoring expired declarations only serves to obscure the full view over an MPs interests and earnings.

</section>

<section aria-labelledby="aggregation">
<h2 id="aggregation">How we aggregate declaration funders</h2>

Declaration data from the Interests API contains inconsistencies that make it difficult to aggregate the data. For example, MPs receiving money from Google have used several aliases:
    - Google
    - Google UK Ltd
    - Google UK Limited

To make our data useful to you, we group probable aliases together and document these as _'recorded names'_. See [Google's page](https://exposed.today/funders/01a15480-2a91-7749-9b2a-c87b5847fe02?q=google) for an example.

We use several data points to connect funders together:
    - Company numbers/legal identifiers
    - Address
    - Names and explicitly mentioned aliases

It's possible that MPs make mistakes when entering declaration records. Company numbers may be incorrect or address may differ slightly. We had to decide whether to take a strict or lax approach to aggregating companies with malformed data. We opted for a relaxed approach. This means that we match on similar addresses and ignore conflicting IDs if there is an address match. A more relaxed approach means there are more cohesive groupings of funders. We think this makes analysis easier. With stricter rules, search queries like 'BBC' or 'Google' may return several of individual donors which is non-sensical.

For some wider more common cases we have configured aliases in our application. In the case of GB News, for instance, grouping based on the data provided in declarations would not have been possible. If you find more cases where obvious groupings have been missed, please let us know.

A declaration can name a donor, a payer and an ultimate payer. These roles can refer to different people or organisations. We use the declaration's evidence to attribute each payment once, rather than counting it under every named role.

</section>

<section aria-labelledby="reuse">
<h2 id="reuse">Licence and citation</h2>

To support the project, please cite Exposed and link to [the project](https://exposed.today) when publishing work that uses it. We suggest including the date you accessed the data, since declarations and matches can change.

Parliamentary data is available under the [Open Parliament Licence v3.0](https://www.parliament.uk/site-information/copyright-parliament/open-parliament-licence/). When reusing it, include the required attribution:

> Contains Parliamentary information licensed under the Open Parliament Licence v3.0.

</section>
