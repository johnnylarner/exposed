<script lang="ts">
  import PeriodSelector from "../../PeriodSelector.svelte";
  import { page, navigating } from "$app/state";
  import AppShell from "../../AppShell.svelte";
  import {
    ViewLinks,
    ViewLink,
    ContentChange,
    RankingList,
    Notice,
  } from "../../lib/ui";
  import {
    rankingCategories,
    rankingRows,
    type RankingCategory,
  } from "../../lib/rankings";
  import type { PageData } from "./$types";
  let { data }: { data: PageData } = $props();
  const selected = $derived(data.selected);
  function categoryHref(category: RankingCategory) {
    const url = new URL(page.url.href);
    url.searchParams.set("category", category);
    url.searchParams.set("period", data.period);
    return url.pathname + url.search;
  }
  const rows = $derived(
    data.rankings ? rankingRows(data.rankings, selected) : [],
  );
</script>

<svelte:head><title>Rankings | Exposed</title></svelte:head>
<AppShell target="rankings" current="rankings">
  <main id="rankings" tabindex="-1" aria-busy={Boolean(navigating.to)}>
    {#if navigating.to && navigating.to.url.pathname !== page.url.pathname}<h1>
        Rankings
      </h1>
    {:else if data.rankings}
      <h1>Rankings</h1>
      <div class="toolbar">
        <ViewLinks label="Ranking category" wrap>
          {#each rankingCategories as category}
            <ViewLink
              href={categoryHref(category.id)}
              data-sveltekit-reset={false}
              aria-current={selected === category.id ? "page" : undefined}
              >{category.label}</ViewLink
            >
          {/each}
        </ViewLinks>
        <PeriodSelector period={data.period} variant="dropdown" />
      </div>
      <ContentChange changeKey={data.period}>
        <div class="board">
          {#if selected === "parties"}<p class="party-context">
              Support declared by MPs, grouped by their latest party.
            </p>{/if}
          <RankingList
            {rows}
            label={`${rankingCategories.find((category) => category.id === selected)?.label} rankings`}
          />
          {#if rows.length === 0}<p class="empty">
              No declarations found.
            </p>{/if}
        </div>
      </ContentChange>
    {:else}<h1>Rankings</h1>
      <Notice role="alert">{data.error}</Notice>{/if}
  </main>
</AppShell>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    margin-block: var(--space-6) var(--space-4);
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
    .toolbar {
      flex-direction: column;
      align-items: stretch;
    }
  }
</style>
