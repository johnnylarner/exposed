<script lang="ts">
  import { Icon, IdentityHeader, ViewLinks, ViewLink, Badge } from "./lib/ui";
  import { page } from "$app/state";
  import { funderHref, type FunderDetails } from "./lib/entity-details";
  import { funderKindLabel } from "./lib/search";
  import posthog from "posthog-js";
  let {
    funder,
    view,
  }: { funder: FunderDetails["funder"]; view: "summary" | "history" } =
    $props();
</script>

<IdentityHeader name={funder.name}
  >{#snippet kind()}<Badge variant="profile"
      ><Icon name="coin" size={20} />Funder</Badge
    >{/snippet}{#snippet metadata()}
    {funderKindLabel(funder.funderKind)}{#if funder.companyNumber}<span
        >Company number {funder.companyNumber}</span
      >{/if}
  {/snippet}{#snippet navigation()}<ViewLinks label="Funder views">
      <ViewLink
        href={funderHref(funder.id, "summary", page.url.searchParams)}
        aria-current={view === "summary" ? "page" : undefined}
        onclick={() =>
          posthog.capture("entity_view_selected", {
            entity_type: "funder",
            selected_view: "summary",
          })}>Summary</ViewLink
      >
      <ViewLink
        href={funderHref(funder.id, "history", page.url.searchParams)}
        aria-current={view === "history" ? "page" : undefined}
        onclick={() =>
          posthog.capture("entity_view_selected", {
            entity_type: "funder",
            selected_view: "history",
          })}>Funding history</ViewLink
      >
    </ViewLinks>{/snippet}</IdentityHeader
>
