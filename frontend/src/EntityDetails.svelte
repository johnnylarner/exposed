<script lang="ts">
  import { Icon, MessageState, Spinner, Button, ButtonLink } from "./lib/ui";
  import { summarizeMember } from "./lib/member-summary";
  import MemberIdentity from "./MemberIdentity.svelte";
  import MemberSummary from "./MemberSummary.svelte";
  import MemberHistory from "./MemberHistory.svelte";
  import FunderIdentity from "./FunderIdentity.svelte";
  import FunderSummary from "./FunderSummary.svelte";
  import FunderHistory from "./FunderHistory.svelte";
  import LordsIdentity from "./LordsIdentity.svelte";
  import LordsSummary from "./LordsSummary.svelte";
  import LordsHistory from "./LordsHistory.svelte";
  import { backToSearch, readDetailRoute } from "./lib/entity-details";
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
  const destination = $derived(
    navigating.to ? readDetailRoute(navigating.to.url.pathname) : null,
  );
  const sameEntity = $derived.by(() => {
    if (!destination) return true;
    if (
      (destination.kind === "member" && route.kind === "member") ||
      (destination.kind === "funder" && route.kind === "funder") ||
      (destination.kind === "lord" && route.kind === "lord")
    )
      return destination.id.toLowerCase() === route.id.toLowerCase();
    return false;
  });
  const loading = $derived(!sameEntity || retrying);
  const isLord = $derived(model.entity === "lord");
  const backHref = $derived(backToSearch(page.url.searchParams));
</script>

<main
  id={isLord ? "lords" : "details"}
  class="detail-page"
  class:member-page={route.kind === "member"}
  class:funder-page={route.kind === "funder"}
  class:lord-page={isLord}
  aria-busy={Boolean(navigating.to) || retrying}
>
  <ButtonLink variant="quiet" size="inline" class="back-link" href={backHref}
    >Back to search</ButtonLink
  >
  {#if loading}
    <MessageState
      title={isLord ? "Loading registered interests" : "Loading details"}
      level={1}
      variant="detail"
      role="status"><Spinner /></MessageState
    >
  {:else if model.result.kind === "not-found"}
    <MessageState
      title={isLord ? "Member not found" : "Entity not found"}
      level={1}
      variant="detail"
    >
      {#if !isLord}<p>
          This MP, Lord or funder is not in the stored register. Search for
          another name.
        </p>{/if}
      <ButtonLink variant="quiet" href={backHref}
        >Search MPs, Lords and funders</ButtonLink
      >
    </MessageState>
  {:else if model.result.kind === "error"}
    <MessageState
      title={isLord
        ? "Registered interests couldn’t load"
        : "Details couldn’t load"}
      level={1}
      variant="detail"
      role="alert"
    >
      <p>{model.result.message}</p>
      <Button onclick={retry}><Icon name="retry" size={16} />Try again</Button>
    </MessageState>
  {:else if model.entity === "member" && model.result.kind === "member"}
    {@const details = model.result.details}
    {@const summary = summarizeMember(details)}
    <MemberIdentity member={details.member} view={model.route.view} />
    {#if model.route.view === "history"}
      <MemberHistory {details} />
    {:else}
      <MemberSummary {summary} period={details.period} />
    {/if}
  {:else if model.entity === "funder" && model.result.kind === "funder"}
    {@const details = model.result.details}
    <FunderIdentity funder={details.funder} view={model.route.view} />
    {#if model.route.view === "history"}
      <FunderHistory {details} />
    {:else}
      <FunderSummary {details} />
    {/if}
  {:else if model.entity === "lord" && model.result.kind === "lord"}
    <LordsIdentity
      member={model.result.details.member}
      view={model.route.view}
    />
    {#if model.route.view === "history"}
      <LordsHistory details={model.result.details} />
    {:else}
      <LordsSummary details={model.result.details} />
    {/if}
  {/if}
</main>

<style>
  .lord-page {
    --accent: var(--accent-lord);
  }
</style>
