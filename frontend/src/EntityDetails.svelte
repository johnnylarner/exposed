<script lang="ts">
  import { summarizeMember } from "./lib/member-summary";
  import MemberIdentity from "./MemberIdentity.svelte";
  import MemberSummary from "./MemberSummary.svelte";
  import MemberHistory from "./MemberHistory.svelte";
  import Icon from "./lib/Icon.svelte";
  import FunderIdentity from "./FunderIdentity.svelte";
  import FunderSummary from "./FunderSummary.svelte";
  import FunderHistory from "./FunderHistory.svelte";
  import { backToSearch, type DetailState } from "./lib/entity-details";
  import { page, navigating } from "$app/state";
  import { invalidate } from "$app/navigation";
  import type { PageModel } from "./lib/page";
  let { model }: { model: Extract<PageModel, { kind: "detail" }> } = $props();
  const route = $derived(model.route);
  let retrying = $state(false);
  async function retry() {
    retrying = true;
    try {
      await invalidate("exposed:page");
    } finally {
      retrying = false;
    }
  }
  const detailState = $derived<DetailState>(
    navigating.to || retrying ? { kind: "loading" } : model.result,
  );
  const backHref = $derived(backToSearch(page.url.searchParams));
</script>

<main
  id="details"
  class="detail-page"
  class:member-page={route.kind === "member"}
  class:funder-page={route.kind === "funder"}
  aria-busy={detailState.kind === "loading"}
>
  <a class="back-link" href={backHref}>Back to search</a>
  {#if detailState.kind === "loading"}
    <div class="detail-message" role="status">
      <span class="loading-indicator" aria-hidden="true"></span>
      <h1>Loading details</h1>
    </div>
  {:else if detailState.kind === "not-found"}
    <div class="detail-message">
      <h1>Entity not found</h1>
      <p>
        This MP or funder is not in the stored register. Search for another
        name.
      </p>
      <a class="text-button" href={backHref}>Search MPs and funders</a>
    </div>
  {:else if detailState.kind === "error"}
    <div class="detail-message" role="alert">
      <h1>Details couldn’t load</h1>
      <p>{detailState.message}</p>
      <button class="retry-button" onclick={retry}
        ><Icon name="retry" size={16} />Try again</button
      >
    </div>
  {:else if detailState.kind === "member"}
    {@const details = detailState.details}
    {@const summary = summarizeMember(details)}
    <MemberIdentity
      member={details.member}
      total={summary.total}
      view={route.kind === "member" ? route.view : "summary"}
    />
    {#if route.kind === "member" && route.view === "history"}<MemberHistory
        {details}
      />{:else}<MemberSummary {details} {summary} />{/if}
  {:else}
    {@const details = detailState.details}
    <FunderIdentity
      {details}
      view={route.kind === "funder" ? route.view : "summary"}
    />
    {#if route.kind === "funder" && route.view === "history"}
      <FunderHistory {details} />
    {:else}
      <FunderSummary {details} />
    {/if}
  {/if}
</main>
