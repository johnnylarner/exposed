<script lang="ts">
  import AppShell from "../../AppShell.svelte";
  import {
    ResultRow,
    CategoryWheel,
    Notice,
    IconIndicator,
  } from "../../lib/ui";
  import {
    updateMonth,
    updateDate,
    updateCategories,
    type UpdateCategory,
  } from "../../lib/updates";
  import type { PageData } from "./$types";
  let { data }: { data: PageData } = $props();
  let selected = $state<UpdateCategory>("members");
  const rows = $derived(data.updates?.[selected] ?? []);
  const category = $derived(updateCategories[selected]);
</script>

<svelte:head><title>Updates | Exposed</title></svelte:head>
<AppShell target="updates" current="updates">
  <main id="updates" tabindex="-1">
    {#if data.updates}
      <div class="updates-intro">
        <div>
          <h1>Updates</h1>
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
            {selected}
            label="Updates category"
            onselect={(category) => (selected = category)}
          />
        </div>
      </div>
      {#key selected}
        <div class="board" style={`--accent: var(--accent-${category.accent})`}>
          {#if rows.length}
            <ul class="results-list" aria-label={`${category.label} updates`}>
              {#each rows as row (row.id)}
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
          {:else}<p class="empty">No declarations found.</p>{/if}
        </div>
      {/key}
    {:else}<h1>Updates</h1>
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
