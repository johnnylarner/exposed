# Exposed

Where do MPs get their money from?

Exposed makes UK parliamentary declarations of financial interests easier to
search, browse, and interpret. The goal is to help people understand what MPs
declare and who provides their financial support, without having to piece
together individual register entries by hand.

## What the project is working towards

The project starts with questions people want to answer:

- What financial interests has an MP declared?
- Which people and organisations provide support to MPs?
- Which MPs receive support from the same funder?
- How do those declarations change over time?

Answering these questions requires more than a name search. Declarations contain
different kinds of interests, incomplete amounts, and funders recorded under
different names. Exposed aims to connect related records while preserving the
source information and the limits of what it tells us.

## What you can explore today

Search for an MP or funder by name, then open their detail page. Search supports
approximate name matching to help with misspellings and recorded name variants.

An MP's page brings together their profile, declared amounts, category
breakdowns, and recent declarations. Their declaration history lets you browse
all stored declarations and filter by registration date.

A funder's page shows recorded names and declared support across recipients.
Recipient and party summaries help you explore how that support is distributed.

The application works with data imported into its database. Its coverage depends
on the records fetched and loaded, so it does not establish a complete picture
of an MP's finances.

## Making declarations interpretable

Declared amounts can describe payments, rates, loans, gifts, or other benefits.
They are not a measure of money earned. Support linked to an MP can also be
received by a party organisation rather than paid to the MP personally.

Exposed preserves exact decimal amounts and currencies. Missing amounts remain
distinct from zero, and totals show when the data is incomplete. GBP totals do
not convert or include foreign currencies.

Registration dates describe when an interest was registered. They do not
necessarily describe when money changed hands. Party summaries use recipients'
latest stored party, which may differ from their party when support was received.

Funder resolution groups observations believed to refer to the same person or
organisation. Some rules accept matching donor names, so groups can include
false matches. The pipeline retains raw captures, cleaned observations, and
resolution evidence so that those decisions can be inspected.

## How the project fits together

The Rust application in `exposed/` fetches parliamentary data, cleans declarations,
resolves funder identities, and loads the searchable PostgreSQL database. It also
provides the HTTP API and CLI. The SvelteKit application in `frontend/` provides
search and detail pages.

The declaration pipeline runs through `fetch`, `clean`, `resolve`, and `load`.
Keeping these stages separate lets contributors reprocess captured data without
fetching it again.

## Git hooks

Enable the repository hooks after cloning:

```sh
git config core.hooksPath .githooks
```

Commit subjects use `type(optional scope)!: description`, for example
`feat(search): add funder filtering`. The scope and `!` are optional. Types
contain letters, and descriptions must contain non-whitespace text. Only the
subject is checked; commit bodies are unrestricted.

Run the hook regression tests with `sh scripts/test-commit-msg.sh`.

## Further reading

The CLI documents its commands through `exposed --help` and subcommand help.
These references explain the data and deployment:

- [Declaration cleaning](docs/declaration-cleaning.md) describes funding occurrences and funder observations.
- [Funder resolution](docs/funder-resolution-design.md) explains the identity and evidence policy.
- [Resolution guide](resolution/README.md) covers configuration and execution.
- [Entity details](docs/entity-details.md) describes API responses and detail views.
- [Database guide](db/README.md) covers schema migrations.
- [Production image](docs/production-image.md) covers builds and deployment.
- [Verification skill](.agents/skills/verify/SKILL.md) compares snapshot-backed worktrees with live production.

## Data licence

Contains Parliamentary information licensed under the
[Open Parliament Licence v3.0](https://www.parliament.uk/site-information/copyright-parliament/open-parliament-licence/).

### Feedback email

The About form sends email through
[Resend](https://resend.com/docs/api-reference/emails/send-email) from the
SvelteKit server. Set both `RESEND_API_KEY` and `FEEDBACK_TO_EMAIL` in the
frontend server environment. The fixed sender is `onboarding@resend.dev`, which
can only deliver to the Resend account email. Set `FEEDBACK_TO_EMAIL` to that
address. The visitor's address is used only as the reply-to address. Without
both required values, submissions report that feedback is temporarily
unavailable.

The [Node adapter](https://svelte.dev/docs/kit/adapter-node) assumes HTTPS. When a trusted reverse proxy forwards the request
scheme, set `PROTOCOL_HEADER` to that header's name (for example,
`x-forwarded-proto`). The proxy must overwrite this header so native form
submissions pass SvelteKit's origin check with the correct scheme.
