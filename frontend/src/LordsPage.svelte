<script lang="ts">
  import { invalidate } from "$app/navigation";
  import { navigating, page } from "$app/state";
  import {
    backToSearch,
    lordHref,
    readDetailRoute,
  } from "./lib/entity-details";
  import {
    currentLordsCategories,
    groupLordsCategories,
    sourceCalendarDate,
    type RegisteredInterest,
  } from "./lib/lords";
  import type { PageModel } from "./lib/page";
  import { shareholdingSummary } from "./lib/lords-shareholdings";
  import {
    Badge,
    Button,
    ButtonLink,
    Icon,
    IdentityHeader,
    MessageState,
    RegisteredInterestItem,
    SectionHeading,
    Spinner,
    ViewLinks,
    ViewLink,
  } from "./lib/ui";
  import posthog from "posthog-js";
  import LordsHistory from "./LordsHistory.svelte";
  let { model }: { model: Extract<PageModel, { kind: "lord" }> } = $props();
  let retrying = $state(false);
  const backHref = $derived(backToSearch(page.url.searchParams));
  const destination = $derived(
    navigating.to ? readDetailRoute(navigating.to.url.pathname) : null,
  );
  const loading = $derived(
    retrying ||
      Boolean(
        destination &&
        (destination.kind !== "lord" || destination.id !== model.id),
      ),
  );
  const groups = $derived(
    model.result.kind === "lord"
      ? groupLordsCategories(
          currentLordsCategories(model.result.details.categories),
        )
      : [],
  );
  const shareholdings = $derived(
    groups.filter((category) => category.placement === "shareholdings"),
  );
  const otherInterests = $derived(
    groups.filter((category) => category.placement === "other"),
  );
  async function retry() {
    retrying = true;
    try {
      await invalidate("exposed:page");
    } finally {
      retrying = false;
    }
  }
</script>

{#snippet interestItems(
  interests: RegisteredInterest[],
  companyLabels: boolean,
)}
  {#each interests as interest}
    <RegisteredInterestItem
      text={companyLabels
        ? shareholdingSummary(interest.text).text
        : (interest.text ?? "Interest text unavailable")}
      status={{ kind: "active" }}
    >
      {#if interest.children.length}
        <ul class={["interest-list", companyLabels && "compact-list"]}>
          {@render interestItems(interest.children, companyLabels)}
        </ul>
      {/if}
    </RegisteredInterestItem>
  {/each}
{/snippet}

<main
  id="lords"
  class="detail-page lord-page"
  aria-busy={Boolean(navigating.to) || retrying}
>
  <ButtonLink variant="quiet" size="inline" class="back-link" href={backHref}
    >Back to search</ButtonLink
  >
  {#if loading}
    <MessageState
      title="Loading registered interests"
      level={1}
      variant="detail"
      role="status"><Spinner /></MessageState
    >
  {:else if model.result.kind === "not-found"}
    <MessageState title="Member not found" level={1} variant="detail">
      <ButtonLink variant="quiet" href={backHref}
        >Search MPs, Lords and funders</ButtonLink
      >
    </MessageState>
  {:else if model.result.kind === "error"}
    <MessageState
      title="Registered interests couldn’t load"
      level={1}
      variant="detail"
      role="alert"
    >
      <p>{model.result.message}</p>
      <Button onclick={retry}><Icon name="retry" size={16} />Try again</Button>
    </MessageState>
  {:else}
    {@const member = model.result.details.member}
    {@const memberMetadata = [
      member.partyName,
      member.membershipFrom,
      member.membershipStatus,
    ].filter((value) => value !== null && value !== "")}
    <IdentityHeader name={member.name}>
      {#snippet kind()}<Badge variant="profile"
          ><Icon name="portcullis" size={20} />Lords</Badge
        >{/snippet}
      {#snippet metadata()}
        {#each memberMetadata as value, index}
          {#if index === 0}{value}{:else}<span>{value}</span>{/if}
        {/each}
        {#if member.membershipEndDate}
          {#snippet ended(date: string)}Membership ended <time datetime={date}
              >{sourceCalendarDate(date)}</time
            >{/snippet}
          {#if memberMetadata.length}<span
              >{@render ended(member.membershipEndDate)}</span
            >{:else}{@render ended(member.membershipEndDate)}{/if}
        {/if}
      {/snippet}
      {#snippet navigation()}
        <ViewLinks label="Lords views">
          <ViewLink
            href={lordHref(model.id, "summary", page.url.searchParams)}
            aria-current={model.view === "summary" ? "page" : undefined}
            onclick={() =>
              posthog.capture("entity_view_selected", {
                entity_type: "lord",
                selected_view: "summary",
              })}>Summary</ViewLink
          >
          <ViewLink
            href={lordHref(model.id, "history", page.url.searchParams)}
            aria-current={model.view === "history" ? "page" : undefined}
            onclick={() =>
              posthog.capture("entity_view_selected", {
                entity_type: "lord",
                selected_view: "history",
              })}>Interest history</ViewLink
          >
        </ViewLinks>
      {/snippet}
    </IdentityHeader>
    {#if model.view === "history"}
      <LordsHistory details={model.result.details} />
    {:else}
      <div class="interest-columns">
        <section aria-labelledby="shareholdings">
          <SectionHeading id="shareholdings" title="Shareholdings" />
          {#if shareholdings.length === 0}<p class="empty-category">
              No interests supplied.
            </p>{:else}
            <ul class="interest-list compact-list">
              {#each shareholdings as category}
                {@render interestItems(category.interests, true)}
              {/each}
            </ul>
          {/if}
        </section>
        <section aria-labelledby="registered-interests">
          <SectionHeading
            id="registered-interests"
            title="Registered interests"
          />
          {#if otherInterests.length === 0}<p class="empty-category">
              No interests supplied.
            </p>{/if}
          {#each otherInterests as category, index}
            <section
              class="interest-category"
              aria-labelledby={`registered-interests-category-${index}`}
            >
              <h3 id={`registered-interests-category-${index}`}>
                {category.name ?? `Category ${category.sourceId}`}
              </h3>
              <ul class="interest-list">
                {@render interestItems(category.interests, false)}
              </ul>
            </section>
          {/each}
        </section>
      </div>
    {/if}
  {/if}
</main>

<style>
  .lord-page {
    --accent: var(--accent-lord);
  }
  .interest-columns {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 2fr);
    gap: var(--space-8);
    margin-top: var(--space-8);
  }
  .interest-columns > section {
    min-width: 0;
  }
  .interest-category {
    margin-top: var(--space-6);
  }
  h3 {
    margin: 0 0 var(--space-3);
    font-size: var(--text-body);
    line-height: 1.6;
    font-weight: 650;
    overflow-wrap: anywhere;
  }
  .interest-list {
    margin: 0;
    padding-left: var(--space-6);
    display: grid;
    gap: var(--space-3);
  }
  .compact-list {
    padding-left: 0;
    list-style: none;
  }
  section > .compact-list {
    margin-top: var(--space-6);
  }
  .interest-list .interest-list {
    margin-top: var(--space-3);
    padding-left: var(--space-4);
  }
  .empty-category {
    color: var(--muted);
    font-size: var(--text-body);
  }
  @media (max-width: 760px) {
    .interest-columns {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
