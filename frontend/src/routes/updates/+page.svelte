<script lang="ts">
  import { tick } from "svelte";
  import AppShell from "../../AppShell.svelte";
  import {
    Button,
    ResultRow,
    CategoryWheel,
    Notice,
    IconIndicator,
  } from "../../lib/ui";
  import {
    updateMonth,
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
        <div>
          <h1>Latest declarations</h1>
          <p class="month">
            <time datetime={data.updates.monthStart.slice(0, 7)}
              >{updateMonth(data.updates.monthStart)}</time
            >
          </p>
        </div>
        <div class="selector">
          <CategoryWheel
            options={Object.values(updateCategories)}
            size={220}
            selected={view.category}
            label="Latest declarations category"
            onselect={selectCategory}
          />
        </div>
      </div>
      <div class="sort-order" role="group" aria-label="Sort order">
        <Button
          size="compact"
          variant={view.sort === "newest" ? "primary" : "secondary"}
          aria-pressed={view.sort === "newest"}
          onclick={() => selectSort("newest")}>Newest first</Button
        >
        <Button
          size="compact"
          variant={view.sort === "oldest" ? "primary" : "secondary"}
          aria-pressed={view.sort === "oldest"}
          onclick={() => selectSort("oldest")}>Oldest first</Button
        >
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
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-6);
    margin-bottom: var(--space-6);
  }
  .selector {
    flex: none;
  }
  .sort-order {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
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
    .updates-intro {
      flex-direction: column;
      align-items: flex-start;
    }
    .selector {
      align-self: center;
    }
    .registered {
      grid-column: 2;
      text-align: left;
    }
  }
</style>
