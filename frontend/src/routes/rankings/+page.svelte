<script lang="ts">
  import PeriodSelector from "../../PeriodSelector.svelte";
  import { page, navigating } from "$app/state";
  import { goto } from "$app/navigation";
  import AppShell from "../../AppShell.svelte";
  import { CategoryWheel, RankingList, Notice } from "../../lib/ui";
  import {
    rankingCategories,
    rankingRows,
    type RankingCategory,
  } from "../../lib/rankings";
  import type { PageData } from "./$types";
  let { data }: { data: PageData } = $props();
  const selected = $derived(data.selected);
  function selectCategory(category: RankingCategory) {
    const url = new URL(page.url.href);
    url.searchParams.set("category", category);
    url.searchParams.set("period", data.period);
    void goto(url.pathname + url.search);
  }
  const rows = $derived(
    data.rankings ? rankingRows(data.rankings, selected) : [],
  );
</script>

<svelte:head><title>Rankings | Exposed</title></svelte:head>
<AppShell target="rankings" current="rankings">
  <main id="rankings" tabindex="-1">
    {#if navigating.to}<h1>Rankings</h1>
    {:else if data.rankings}
      <div class="ranking-intro">
        <div>
          <h1>Rankings</h1>
          <PeriodSelector period={data.period} />
        </div>
        <CategoryWheel
          options={rankingCategories}
          size={220}
          {selected}
          label="Ranking category"
          onselect={selectCategory}
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
