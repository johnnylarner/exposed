<script lang="ts">
  import { invalidate } from "$app/navigation";
  import { navigating, page } from "$app/state";
  import { backToSearch } from "./lib/entity-details";
  import {
    groupLordsCategories,
    sourceCalendarDate,
    type RegisteredInterest,
  } from "./lib/lords";
  import type { PageModel } from "./lib/page";
  import {
    Badge,
    Button,
    ButtonLink,
    Icon,
    IdentityHeader,
    MessageState,
    RegisteredInterestCard,
    SectionHeading,
    Spinner,
  } from "./lib/ui";
  let { model }: { model: Extract<PageModel, { kind: "lord" }> } = $props();
  let retrying = $state(false);
  const backHref = $derived(backToSearch(page.url.searchParams));
  const loading = $derived(
    retrying ||
      Boolean(
        navigating.to && navigating.to.url.pathname !== `/lords/${model.id}`,
      ),
  );
  const groups = $derived(
    model.result.kind === "lord"
      ? groupLordsCategories(model.result.details.categories)
      : [],
  );
  const columns = $derived([
    {
      id: "shareholdings",
      title: "Shareholdings",
      categories: groups.filter(
        (category) => category.placement === "shareholdings",
      ),
    },
    {
      id: "registered-interests",
      title: "Registered interests",
      categories: groups.filter((category) => category.placement === "other"),
    },
  ]);
  async function retry() {
    retrying = true;
    try {
      await invalidate("exposed:page");
    } finally {
      retrying = false;
    }
  }
</script>

{#snippet interestTree(interests: RegisteredInterest[])}
  <ol class="interest-list">
    {#each interests as interest}
      {#snippet metadata()}
        {#if interest.createdWhen !== null}<span
            >Created <time datetime={interest.createdWhen}
              >{sourceCalendarDate(interest.createdWhen)}</time
            ></span
          >{/if}
        {#if interest.lastAmendedWhen !== null}<span
            >Amended <time datetime={interest.lastAmendedWhen}
              >{sourceCalendarDate(interest.lastAmendedWhen)}</time
            ></span
          >{/if}
      {/snippet}
      <RegisteredInterestCard
        text={interest.text ?? "Interest text unavailable"}
        status={interest.deletedWhen === null
          ? { kind: "active" }
          : {
              kind: "deleted",
              datetime: interest.deletedWhen,
              date: sourceCalendarDate(interest.deletedWhen),
            }}
        correction={interest.isCorrection}
        metadata={interest.createdWhen !== null ||
        interest.lastAmendedWhen !== null
          ? metadata
          : undefined}
      >
        {#if interest.children.length}{@render interestTree(
            interest.children,
          )}{/if}
      </RegisteredInterestCard>
    {/each}
  </ol>
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
    </IdentityHeader>
    <div class="interest-columns">
      {#each columns as column}
        <section aria-labelledby={column.id}>
          <SectionHeading id={column.id} title={column.title} />
          {#if column.categories.length === 0}<p class="empty-category">
              No interests supplied.
            </p>{/if}
          {#each column.categories as category, index}
            <section
              class="interest-category"
              aria-labelledby={`${column.id}-category-${index}`}
            >
              <h3 id={`${column.id}-category-${index}`}>
                {category.name ?? `Category ${category.sourceId}`}
              </h3>
              {#if category.interests.length}{@render interestTree(
                  category.interests,
                )}{:else}<p class="empty-category">
                  No interests supplied.
                </p>{/if}
            </section>
          {/each}
        </section>
      {/each}
    </div>
  {/if}
</main>

<style>
  .lord-page {
    --accent: var(--accent-lord);
  }
  .interest-columns {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
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
    padding: 0;
    list-style: none;
    display: grid;
    gap: var(--space-3);
  }
  .interest-list .interest-list {
    margin-top: var(--space-3);
    padding-left: var(--space-4);
  }
  .interest-list span + span {
    margin-left: var(--space-3);
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
