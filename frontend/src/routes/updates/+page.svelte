<script lang="ts">
  import { tick } from "svelte";
  import AppShell from "../../AppShell.svelte";
  import {
    Button,
    ResultRow,
    ViewButtons,
    ViewButton,
    Select,
    Notice,
    IconIndicator,
  } from "../../lib/ui";
  import {
    updateMonth,
    updateSortOptions,
    updateDate,
    updateCategories,
    sortUpdates,
    type UpdateCategory,
    type UpdateSort,
    type UpdateView,
  } from "../../lib/updates";
  import type { PageData } from "./$types";
  let { data }: { data: PageData } = $props();
  const batchSize = 10;
  let view = $state<UpdateView>({
    category: "members",
    sort: "newest",
    visible: batchSize,
  });
  let board = $state<HTMLDivElement>();
  const rows = $derived(
    sortUpdates(data.updates?.[view.category] ?? [], view.sort),
  );
  const visibleRows = $derived(rows.slice(0, view.visible));
  const category = $derived(updateCategories[view.category]);
  function selectCategory(category: UpdateCategory): void {
    if (category !== view.category)
      view = { ...view, category, visible: batchSize };
  }
  function selectSort(sort: UpdateSort): void {
    if (sort !== view.sort) view = { ...view, sort, visible: batchSize };
  }
  async function showMore(): Promise<void> {
    const firstNewRow = view.visible;
    view = { ...view, visible: view.visible + batchSize };
    if (view.visible >= rows.length) {
      await tick();
      board
        ?.querySelectorAll<HTMLAnchorElement>(".entity-name")
        [firstNewRow]?.focus();
    }
  }
</script>

<svelte:head><title>Latest declarations | Exposed</title></svelte:head>
<AppShell target="updates" current="updates">
  <main id="updates" tabindex="-1">
    {#if data.updates}
      <div class="updates-intro">
        <h1>Latest declarations</h1>
        <p class="month">
          <time datetime={data.updates.monthStart.slice(0, 7)}
            >{updateMonth(data.updates.monthStart)}</time
          >
        </p>
      </div>
      <div class="toolbar">
        <ViewButtons label="Latest declarations category">
          {#each Object.values(updateCategories) as category}
            <ViewButton
              selected={view.category === category.id}
              onclick={() => selectCategory(category.id)}
              >{category.label}</ViewButton
            >
          {/each}
        </ViewButtons>
        <Select
          label="Sort order"
          options={updateSortOptions}
          value={view.sort}
          onselect={selectSort}
        />
      </div>
      {#key `${view.category}-${view.sort}`}
        <div
          bind:this={board}
          class="board"
          style={`--accent: var(--accent-${category.accent})`}
        >
          {#if rows.length}
            <ul
              class="results-list"
              aria-label={`${category.label} latest declarations`}
            >
              {#each visibleRows as row (row.id)}
                <ResultRow>
                  <IconIndicator icon={category.icon} label={category.kind} />
                  <div class="entity-text">
                    <a
                      class="entity-name"
                      href={`/${category.prefix}/${row.id}`}>{row.name}</a
                    >
                    <span class="entity-alias"
                      >{row.declarationCount} declaration{row.declarationCount ===
                      1
                        ? ""
                        : "s"}</span
                    >
                  </div>
                  <time class="registered" datetime={row.latestRegistration}
                    >{updateDate(row.latestRegistration)}</time
                  >
                </ResultRow>
              {/each}
            </ul>
            {#if view.visible < rows.length}
              <div class="show-more">
                <Button onclick={showMore}>Show more</Button>
              </div>
            {/if}
          {:else}<p class="empty">No declarations found.</p>{/if}
        </div>
      {/key}
    {:else}<h1>Latest declarations</h1>
      <Notice role="alert">{data.error}</Notice>{/if}
  </main>
</AppShell>

<style>
  .month {
    margin: var(--space-2) 0;
    color: var(--muted);
    font-size: var(--text-heading);
    line-height: 1.5;
  }
  .updates-intro {
    margin-bottom: var(--space-6);
  }
  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    margin-bottom: var(--space-4);
  }
  .show-more {
    margin-top: var(--space-4);
  }
  .board {
    min-width: 0;
    animation: board-enter 250ms ease-out both;
  }
  @keyframes board-enter {
    from {
      transform: translateY(6px);
    }
    to {
      transform: translateY(0);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .board {
      animation: none;
    }
  }
  .entity-name {
    color: var(--accent);
  }
  .registered {
    flex: none;
    color: var(--muted);
    font-size: var(--text-small);
    text-align: right;
  }
  .empty {
    color: var(--muted);
  }
  @media (max-width: 480px) {
    .toolbar {
      flex-direction: column;
      align-items: stretch;
    }
    .registered {
      grid-column: 2;
      text-align: left;
    }
  }
</style>
