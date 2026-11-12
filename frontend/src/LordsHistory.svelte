<script lang="ts">
  import {
    SectionHeading,
    DeclarationList,
    DeclarationCard,
    RegisteredInterestItem,
    Pagination,
  } from "./lib/ui";
  import {
    sourceCalendarDate,
    type LordsDetails,
    type LordsCategory,
    type RegisteredInterest,
  } from "./lib/lords";
  let { details }: { details: LordsDetails } = $props();
  let requestedPage = $state(1);
  const pageSize = 10;
  type InterestCard = {
    category: LordsCategory;
    interest: RegisteredInterest | null;
  };
  const cards = $derived(
    [...details.categories]
      .sort((left, right) => left.sortOrder - right.sortOrder)
      .flatMap<InterestCard>((category) =>
        category.interests.length
          ? category.interests.map((interest) => ({ category, interest }))
          : [{ category, interest: null }],
      ),
  );
  const pageCount = $derived(Math.max(1, Math.ceil(cards.length / pageSize)));
  const page = $derived(Math.min(requestedPage, pageCount));
  const visible = $derived(cards.slice((page - 1) * pageSize, page * pageSize));
  $effect(() => {
    details;
    requestedPage = 1;
  });
</script>

{#snippet interestItems(interests: RegisteredInterest[])}
  {#each interests as interest}
    <RegisteredInterestItem
      text={interest.text ?? "Interest text unavailable"}
      status={interest.deletedWhen === null
        ? { kind: "active" }
        : {
            kind: "deleted",
            datetime: interest.deletedWhen,
            date: sourceCalendarDate(interest.deletedWhen),
          }}
      correction={interest.isCorrection}
    >
      {#snippet metadata()}
        <span class="source-detail">Interest {interest.sourceId}</span>
        {#if interest.createdWhen !== null}<span class="source-detail"
            >Created <time datetime={interest.createdWhen}
              >{interest.createdWhen}</time
            ></span
          >{/if}
        {#if interest.lastAmendedWhen !== null}<span class="source-detail"
            >Amended <time datetime={interest.lastAmendedWhen}
              >{interest.lastAmendedWhen}</time
            ></span
          >{/if}
        {#if interest.deletedWhen !== null}<span class="source-detail"
            >Deleted <time datetime={interest.deletedWhen}
              >{interest.deletedWhen}</time
            ></span
          >{/if}
      {/snippet}
      {#if interest.children.length}<ul class="interest-list">
          {@render interestItems(interest.children)}
        </ul>{/if}
    </RegisteredInterestItem>
  {/each}
{/snippet}

<section aria-labelledby="interest-history">
  <SectionHeading id="interest-history" title="Interest history" />
  <p class="capture">
    Captured <time datetime={details.capturedAt}>{details.capturedAt}</time>
  </p>
  {#if cards.length === 0}<p class="empty-category">
      No interests supplied.
    </p>{:else}
    <DeclarationList label="Registered interest history">
      {#each visible as card}
        <DeclarationCard variant="compact">
          {#snippet heading()}<h3>
              {card.category.name ?? `Category ${card.category.sourceId}`}
            </h3>{/snippet}
          {#snippet registered()}Category {card.category.sourceId}{/snippet}
          {#if card.interest}<ul class="interest-list">
              {@render interestItems([card.interest])}
            </ul>{:else}<p class="empty-category">
              No interests supplied.
            </p>{/if}
        </DeclarationCard>
      {/each}
    </DeclarationList>
    {#if pageCount > 1}<Pagination
        {page}
        {pageCount}
        label="Interest history pages"
        onprevious={() => (requestedPage = page - 1)}
        onnext={() => (requestedPage = page + 1)}
      />{/if}
  {/if}
</section>

<style>
  section {
    margin-top: var(--space-8);
    min-width: 0;
  }
  h3 {
    margin: 0;
    font-size: var(--text-body);
    overflow-wrap: anywhere;
  }
  .interest-list {
    margin: 0;
    padding-left: var(--space-6);
    display: grid;
    gap: var(--space-3);
  }
  .interest-list .interest-list {
    margin-top: var(--space-3);
    padding-left: var(--space-4);
  }
  .source-detail {
    display: block;
  }
  .capture,
  .empty-category {
    color: var(--muted);
    font-size: var(--text-small);
    overflow-wrap: anywhere;
  }
  .capture {
    margin: var(--space-3) 0 var(--space-6);
  }
</style>
