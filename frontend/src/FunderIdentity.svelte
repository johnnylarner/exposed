<script lang="ts">
  import { Icon, IdentityHeader, ViewLinks, ViewLink, Badge } from "./lib/ui";
  import { page } from "$app/state";
  import PeriodSelector from "./PeriodSelector.svelte";
  import DeclaredTotals from "./DeclaredTotals.svelte";
  import { funderHref, type FunderDetails } from "./lib/entity-details";
  import { funderKindLabel } from "./lib/search";
  import posthog from "posthog-js";
  let {
    details,
    view,
  }: { details: FunderDetails; view: "summary" | "history" } = $props();
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
    <div class="identity-totals">
      <PeriodSelector period={details.period} />
      <DeclaredTotals totals={details.declaredTotals} period={details.period} />
    </div>
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

<style>
  .identity-totals {
    grid-column: 1 / -1;
    min-width: 0;
  }
</style>
