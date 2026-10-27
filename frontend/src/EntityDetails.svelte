<script lang="ts">
  import { Icon, MessageState, Spinner, Button, ButtonLink } from "./lib/ui";
  import { summarizeMember } from "./lib/member-summary";
  import MemberIdentity from "./MemberIdentity.svelte";
  import MemberSummary from "./MemberSummary.svelte";
  import MemberHistory from "./MemberHistory.svelte";
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
    (navigating.to && navigating.to.url.pathname !== page.url.pathname) ||
      retrying
      ? { kind: "loading" }
      : model.result,
  );
  const backHref = $derived(backToSearch(page.url.searchParams));
</script>

<main
  id="details"
  class="detail-page"
  class:member-page={route.kind === "member"}
  class:funder-page={route.kind === "funder"}
  aria-busy={Boolean(navigating.to) || retrying}
>
  <ButtonLink variant="quiet" size="inline" class="back-link" href={backHref}
    >Back to search</ButtonLink
  >
  {#if detailState.kind === "loading"}
    <MessageState
      title="Loading details"
      level={1}
      variant="detail"
      role="status"><Spinner /></MessageState
    >
  {:else if detailState.kind === "not-found"}
    <MessageState title="Entity not found" level={1} variant="detail">
      <p>
        This MP or funder is not in the stored register. Search for another
        name.
      </p>
      <ButtonLink variant="quiet" href={backHref}
        >Search MPs and funders</ButtonLink
      >
    </MessageState>
  {:else if detailState.kind === "error"}
    <MessageState
      title="Details couldn’t load"
      level={1}
      variant="detail"
      role="alert"
    >
      <p>{detailState.message}</p>
      <Button onclick={retry}><Icon name="retry" size={16} />Try again</Button>
    </MessageState>
  {:else if detailState.kind === "member"}
    {@const details = detailState.details}
    {@const summary = summarizeMember(details)}
    <MemberIdentity
      member={details.member}
      period={details.period}
      {summary}
      view={route.kind === "member" ? route.view : "summary"}
    />
    {#if route.kind === "member" && route.view === "history"}<MemberHistory
        {details}
      />{:else}<MemberSummary {summary} />{/if}
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
