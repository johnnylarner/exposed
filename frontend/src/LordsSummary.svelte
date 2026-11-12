<script lang="ts">
  import {
    currentLordsCategories,
    groupLordsCategories,
    type LordsDetails,
    type RegisteredInterest,
  } from "./lib/lords";
  import { shareholdingSummary } from "./lib/lords-shareholdings";
  import { RegisteredInterestItem, SectionHeading } from "./lib/ui";
  let { details }: { details: LordsDetails } = $props();
  const groups = $derived(
    groupLordsCategories(currentLordsCategories(details.categories)),
  );
  const shareholdings = $derived(
    groups.filter((category) => category.placement === "shareholdings"),
  );
  const otherInterests = $derived(
    groups.filter((category) => category.placement === "other"),
  );
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
      variant={companyLabels ? "company" : "default"}
    >
      {#if interest.children.length}
        <ul class={["interest-list", companyLabels && "compact-list"]}>
          {@render interestItems(interest.children, companyLabels)}
        </ul>
      {/if}
    </RegisteredInterestItem>
  {/each}
{/snippet}

<div class="interest-columns">
  <section aria-labelledby="shareholdings">
    <SectionHeading id="shareholdings" title="Shareholdings" tone="accent" />
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
    <SectionHeading id="registered-interests" title="Registered interests" />
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

<style>
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
    margin-top: var(--space-8);
  }
  h3 {
    color: var(--accent);
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
    gap: var(--space-4);
  }
  .compact-list {
    gap: 0;
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
