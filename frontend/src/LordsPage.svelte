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
  import { shareholdingSummary } from "./lib/lords-shareholdings";
  import {
    Badge,
    Button,
    ButtonLink,
    Icon,
    IdentityHeader,
    MessageState,
    RegisteredInterestItem,
    type RegisteredInterestPresentation,
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
  presentation: RegisteredInterestPresentation["kind"],
  category: string,
)}
  {#each interests as interest}
    {#snippet metadata()}
      {#if presentation === "compact"}<span class="source-detail"
          >{category}</span
        >{/if}
      {#if interest.createdWhen !== null}<span class="source-detail"
          >Created <time datetime={interest.createdWhen}
            >{sourceCalendarDate(interest.createdWhen)}</time
          ></span
        >{/if}
      {#if interest.lastAmendedWhen !== null}<span class="source-detail"
          >Amended <time datetime={interest.lastAmendedWhen}
            >{sourceCalendarDate(interest.lastAmendedWhen)}</time
          ></span
        >{/if}
    {/snippet}
    <RegisteredInterestItem
      text={interest.text?.trim() ? interest.text : "Interest text unavailable"}
      presentation={presentation === "compact"
        ? { kind: "compact", summary: shareholdingSummary(interest.text).text }
        : { kind: "full" }}
      status={interest.deletedWhen === null
        ? { kind: "active" }
        : {
            kind: "deleted",
            datetime: interest.deletedWhen,
            date: sourceCalendarDate(interest.deletedWhen),
          }}
      correction={interest.isCorrection}
      metadata={presentation === "compact" ||
      interest.createdWhen !== null ||
      interest.lastAmendedWhen !== null
        ? metadata
        : undefined}
    >
      {#if interest.children.length}
        <ul
          class={[
            "interest-list",
            presentation === "compact" && "compact-list",
          ]}
        >
          {@render interestItems(interest.children, presentation, category)}
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
    </IdentityHeader>
    <div class="interest-columns">
      <section aria-labelledby="shareholdings">
        <SectionHeading id="shareholdings" title="Shareholdings" />
        {#if shareholdings.every((category) => category.interests.length === 0)}<p
            class="empty-category"
          >
            No interests supplied.
          </p>{:else}
          <ul class="interest-list compact-list">
            {#each shareholdings as category}
              {@render interestItems(
                category.interests,
                "compact",
                category.name ?? `Category ${category.sourceId}`,
              )}
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
            {#if category.interests.length}
              <ul class="interest-list">
                {@render interestItems(
                  category.interests,
                  "full",
                  category.name ?? `Category ${category.sourceId}`,
                )}
              </ul>
            {:else}<p class="empty-category">No interests supplied.</p>{/if}
          </section>
        {/each}
      </section>
    </div>
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
  .source-detail + .source-detail {
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
