<script lang="ts">
  import FunderDeclarations from "./FunderDeclarations.svelte";
  import { type FunderDetails } from "./lib/entity-details";
  let { details }: { details: FunderDetails } = $props();
  let from = $state("");
  let to = $state("");
  let page = $state(1);
  const pageSize = 10;
  const reversed = $derived(from !== "" && to !== "" && from > to);
  const filtered = $derived(
    reversed
      ? []
      : details.declarations.filter((declaration) => {
          if (!from && !to) return true;
          if (declaration.declaration.registeredAt === null) return false;
          const day = new Date(declaration.declaration.registeredAt)
            .toISOString()
            .slice(0, 10);
          return (!from || day >= from) && (!to || day <= to);
        }),
  );
  const pageCount = $derived(
    Math.max(1, Math.ceil(filtered.length / pageSize)),
  );
  const visible = $derived(
    filtered.slice((page - 1) * pageSize, page * pageSize),
  );
  function resetPage() {
    page = 1;
  }
  function clearDates() {
    from = "";
    to = "";
    resetPage();
  }
</script>

<section aria-labelledby="history-heading">
  <div class="detail-section-heading">
    <h2 id="history-heading">Funding history</h2>
    <p>
      All {details.declarations.length} stored funding declarations, most recent registration
      first.
    </p>
  </div>
  <div class="history-filters">
    <label
      >Registered from<input
        type="date"
        bind:value={from}
        onchange={resetPage}
      /></label
    ><label
      >Registered to<input
        type="date"
        bind:value={to}
        onchange={resetPage}
      /></label
    ><button class="text-button" onclick={clearDates} disabled={!from && !to}
      >Clear dates</button
    >
  </div>
  <p class="filter-context">
    Filter by registration date. Both dates are included.{#if from || to}
      Declarations without a registration date are excluded.{:else}
      Declarations without a registration date appear last.{/if}
  </p>
  {#if reversed}<p class="coverage-note" role="alert">
      The start date must be on or before the end date.
    </p>
  {:else if filtered.length === 0}<p class="empty-details">
      {details.declarations.length === 0
        ? "No funding history is stored for this funder."
        : "No declarations match these dates. Clear or change the dates."}
    </p>
  {:else}<p class="history-result-count">
      Showing {(page - 1) * pageSize + 1} to {Math.min(
        page * pageSize,
        filtered.length,
      )} of {filtered.length} declarations
    </p>
    <FunderDeclarations declarations={visible} />
    <nav class="history-pagination" aria-label="Declaration pages">
      <button
        class="text-button"
        disabled={page === 1}
        onclick={() => (page -= 1)}>Previous</button
      ><span>Page {page} of {pageCount}</span><button
        class="text-button"
        disabled={page === pageCount}
        onclick={() => (page += 1)}>Next</button
      >
    </nav>
  {/if}
</section>
