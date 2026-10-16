<script lang="ts">
  import { page } from "$app/state";
  import { formatPounds } from "./lib/member-summary";
  import { funderHref, type FunderDetails } from "./lib/entity-details";
  import { funderKindLabel } from "./lib/search";
  import Icon from "./lib/Icon.svelte";
  let {
    details,
    view,
  }: { details: FunderDetails; view: "summary" | "history" } = $props();
  const total = $derived(
    details.currencies.find((group) => group.currency === "GBP")?.amount ??
      null,
  );
</script>

<header class="member-identity">
  <div class="member-identity-info">
    <div class="member-identity-title">
      <span class="profile-type"><Icon name="coin" size={20} />Funder</span>
      <h1>{details.funder.name}</h1>
    </div>
    <p>
      {funderKindLabel(details.funder.funderKind)}<span
        >{details.entryCount}
        {details.entryCount === 1 ? "funding entry" : "funding entries"}</span
      >{#if details.funder.companyNumber}<span
          >Company number {details.funder.companyNumber}</span
        >{/if}
    </p>
  </div>
  {#if view === "summary"}<div class="member-value">
      <span>Total funded</span>
      <p class="member-total">{formatPounds(total)}</p>
    </div>{/if}
  <nav class="member-tabs" aria-label="Funder views">
    <a
      href={funderHref(details.funder.id, "summary", page.url.searchParams)}
      aria-current={view === "summary" ? "page" : undefined}>Summary</a
    >
    <a
      href={funderHref(details.funder.id, "history", page.url.searchParams)}
      aria-current={view === "history" ? "page" : undefined}>Funding history</a
    >
  </nav>
</header>
