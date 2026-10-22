<script lang="ts">
  import {
    Icon,
    Metric,
    IdentityHeader,
    ViewLinks,
    ViewLink,
    Badge,
  } from "./lib/ui";
  import { page } from "$app/state";
  import { formatPounds } from "./lib/member-summary";
  import { funderHref, type FunderDetails } from "./lib/entity-details";
  import { funderKindLabel } from "./lib/search";
  import posthog from "posthog-js";
  let {
    details,
    view,
  }: { details: FunderDetails; view: "summary" | "history" } = $props();
  const total = $derived(
    details.currencies.find((group) => group.currency === "GBP")?.amount ??
      null,
  );
</script>

<IdentityHeader name={details.funder.name}
  >{#snippet kind()}<Badge variant="profile"
      ><Icon name="coin" size={20} />Funder</Badge
    >{/snippet}{#snippet metadata()}
    {funderKindLabel(details.funder.funderKind)}<span
      >{details.entryCount}
      {details.entryCount === 1 ? "funding entry" : "funding entries"}</span
    >{#if details.funder.companyNumber}<span
        >Company number {details.funder.companyNumber}</span
      >{/if}
  {/snippet}{#snippet values()}
    {#if view === "summary"}<Metric
        label="Total funded"
        value={formatPounds(total)}
        available={total !== null}
      />{/if}
  {/snippet}{#snippet navigation()}<ViewLinks label="Funder views">
      <ViewLink
        href={funderHref(details.funder.id, "summary", page.url.searchParams)}
        aria-current={view === "summary" ? "page" : undefined}
        onclick={() =>
          posthog.capture("entity_view_selected", {
            entity_type: "funder",
            selected_view: "summary",
          })}>Summary</ViewLink
      >
      <ViewLink
        href={funderHref(details.funder.id, "history", page.url.searchParams)}
        aria-current={view === "history" ? "page" : undefined}
        onclick={() =>
          posthog.capture("entity_view_selected", {
            entity_type: "funder",
            selected_view: "history",
          })}>Funding history</ViewLink
      >
    </ViewLinks>{/snippet}</IdentityHeader
>
