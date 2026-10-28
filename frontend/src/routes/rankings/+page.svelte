<script lang="ts">
  import { tick } from "svelte";
  import type { Rankings } from "../../lib/rankings";
  import PeriodSelector from "../../PeriodSelector.svelte";
  import { page, navigating } from "$app/state";
  import AppShell from "../../AppShell.svelte";
  import {
    Button,
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
  const batchSize = 10;
  let revealed = $state.raw<{
    rankings: Rankings | null;
    category: RankingCategory;
    count: number;
  }>({
    rankings: null,
    category: "members",
    count: batchSize,
  });
  const visibleCount = $derived(
    revealed.rankings === data.rankings && revealed.category === selected
      ? revealed.count
      : batchSize,
  );
  const visibleRows = $derived(rows.slice(0, visibleCount));
  const boardLabel = $derived(
    `${rankingCategories.find((category) => category.id === selected)?.label} rankings`,
  );
  let board = $state<HTMLDivElement>();
  $effect(() => {
    if (revealed.rankings !== data.rankings || revealed.category !== selected) {
      revealed = {
        rankings: data.rankings,
        category: selected,
        count: batchSize,
      };
    }
  });
  async function showMore(event: MouseEvent) {
    if (navigating.to || visibleCount >= rows.length) return;
    const rankings = data.rankings;
    const category = selected;
    const restoreFocus = event.currentTarget === document.activeElement;
    const count = Math.min(visibleCount + batchSize, rows.length);
    revealed = { rankings, category, count };
    if (count === rows.length && restoreFocus) {
      await tick();
      if (data.rankings === rankings && selected === category) {
        board?.focus({ preventScroll: true });
      }
    }
  }
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
        <PeriodSelector period={data.period} />
      </div>
      <ContentChange changeKey={data.period}>
        <div
          class="board"
          bind:this={board}
          tabindex="-1"
          role="region"
          aria-label={boardLabel}
        >
          {#if selected === "parties"}<p class="party-context">
              Support declared by MPs, grouped by their latest party.
            </p>{/if}
          <RankingList rows={visibleRows} label={boardLabel} />
          {#if visibleCount < rows.length}
            <div class="show-more">
              <Button onclick={showMore} disabled={Boolean(navigating.to)}
                >Show more</Button
              >
            </div>
          {/if}
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
  .show-more {
    display: flex;
    justify-content: center;
    margin-top: var(--space-4);
  }
  .board {
    min-width: 0;
  }
  .board:focus-visible {
    outline: var(--focus-width) solid var(--focus-color);
    outline-offset: var(--focus-offset);
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
