---
name: rust-domain-development
description: "Design and change Rust domain types, service boundaries, and serialized contracts in Exposed. Use for Rust features, bug fixes, and refactors that touch those concerns."
---

# Rust domain development

Apply the [repository coding policy](../../../AGENTS.md). Choose the representation
that enforces the required behavior before adding implementation logic.

## Find the owner

Name the incoming data shape, the valid domain states, and the invariant the
change must preserve. Find the existing type that owns that rule. Search its
callers and serialized spellings before creating another representation.

Trace a changed domain type through its producer, encoder, decoder, service,
and persistence mapping. Change the affected consumers in the same task.

## Choose the representation

- Use an enum for a closed vocabulary. Share its serialization and deserialization
  definition across producers and consumers. When a separately versioned external
  contract needs its own type, convert it explicitly into the canonical domain
  type at the boundary.
- Represent mutually exclusive states with an enum carrying the fields each
  state requires. Use newtypes for identifiers with different meanings and
  checked constructors for values whose validity depends on runtime facts.
- Use exhaustive matches without a fallback when variants require different
  behavior. Adding a variant must require a decision at those matches. Do not
  add artificial matches when every variant already supports the same operation.
- Keep transport DTOs only where the external shape differs from the domain
  shape. Their domain-bearing fields still use the canonical types. Reuse an
  existing result record when it already defines the serialized contract.

## Parse at the boundary

Decode external values into domain types before passing them to services.
Reject unknown labels during parsing. Do not parse a domain enum into a string
and then reconstruct its vocabulary with a membership check.

Keep checks for runtime facts such as duplicate IDs, missing references,
artifact digests, and stored timestamps at the responsible boundary. Types
cannot establish facts about files or database rows that have not been read.
Propagate the existing error types instead of hiding failures with defaults.

Keep domain decisions in the domain model and transaction ownership in the
repository. Convert typed values into storage representations at the persistence
boundary using the canonical encoding. Preserve existing service abstractions.

## Fix the structure

When a failure reveals two owners for one rule, identify the canonical owner and
delete the competing definition. Updating both copies leaves the cause intact.
Stay within the affected contract rather than expanding into unrelated cleanup.

In Exposed, the resolution domain owns `IdentityBasis`, `PairReason`, and the
attribution decision types in
[`declaration_resolution.rs`](../../../exposed/src/lib/domain/models/declaration_resolution.rs).
The [declaration loader](../../../exposed/src/lib/outbound/declaration_loading.rs)
must decode those types. It must not maintain a second list of accepted labels.
This applies to other finite domain vocabularies too.

## Verify the contract

Run the relevant behavior checks and the real entry point affected by the change.
For serialized contracts, verify that producer output is readable by the consumer
and that malformed external values are rejected. Preserve the intended wire
format, supported versions, and persistence behavior unless the task changes them.

Check the compiler guarantee directly when the task relies on it. For example,
temporarily add a variant to confirm the semantic matches reject missing handling,
then revert the probe. A passing runtime test does not prove exhaustiveness.

Run formatting and applicable compiler or lint checks. Distinguish diagnostics
introduced by the change from existing failures. Report which invalid state the
types now exclude and which runtime checks remain necessary.
