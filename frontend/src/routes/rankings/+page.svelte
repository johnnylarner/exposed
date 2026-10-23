<script lang="ts">
  import AppShell from "../../AppShell.svelte";
  import { CategoryWheel, RankingList, Notice } from "../../lib/ui";
  import {
    rankingCategories,
    rankingRows,
    type RankingCategory,
  } from "../../lib/rankings";
  import { registrationDate } from "../../lib/entity-details";
  import type { PageData } from "./$types";
  let { data }: { data: PageData } = $props();
  let selected = $state<RankingCategory>("members");
  const rows = $derived(
    data.rankings ? rankingRows(data.rankings, selected) : [],
  );
</script>

<svelte:head><title>Rankings | Exposed</title></svelte:head>
<AppShell target="rankings" current="rankings">
  <main id="rankings" tabindex="-1">
    {#if data.rankings}
      <div class="ranking-intro">
        <div>
          <h1>Rankings</h1>
          <p class="ranking-meta">
            Since {registrationDate(data.rankings.currentTermStart)}
          </p>
          <p class="ranking-meta">
            {selected === "members"
              ? "Declared amounts · GBP"
              : "Donations and support · GBP"}
          </p>
        </div>
        <CategoryWheel
          options={rankingCategories}
          size={220}
          {selected}
          label="Ranking category"
          onselect={(category) => (selected = category)}
        />
      </div>
      <div class="board">
        {#if selected === "parties"}<p class="party-context">
            Support declared by MPs, grouped by their latest party.
          </p>{/if}
        <RankingList
          {rows}
          label={`${rankingCategories.find((category) => category.id === selected)?.label} rankings`}
        />
        {#if rows.length === 0}<p class="empty">No declarations found.</p>{/if}
      </div>
    {:else}<h1>Rankings</h1>
      <Notice role="alert">{data.error}</Notice>{/if}
  </main>
</AppShell>

<style>
  .ranking-meta {
    margin: var(--space-2) 0;
    color: var(--muted);
    font-size: var(--text-small);
  }
  .ranking-intro {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-6);
    margin-bottom: var(--space-6);
  }
  .ranking-intro :global(.category-wheel) {
    flex: none;
  }
  .board {
    min-width: 0;
  }
  .party-context,
  .empty {
    margin: 0 0 var(--space-2);
    color: var(--muted);
    font-size: var(--text-small);
    line-height: 1.6;
  }
  @media (max-width: 480px) {
    .ranking-intro {
      flex-direction: column;
      align-items: flex-start;
    }
    .ranking-intro :global(.category-wheel) {
      align-self: center;
    }
  }
</style>
