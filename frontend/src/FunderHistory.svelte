<script lang="ts">
  import { SectionHeading, Pagination, Button, Field, Notice } from "./lib/ui";
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
  <SectionHeading id="history-heading" title="Funding history">
    <p class="entry-count">
      {details.entryCount}
      {details.entryCount === 1 ? "funding entry" : "funding entries"}
    </p>
    <p>
      {details.declarations.length} funding declarations, most recent registration
      first.
    </p>
  </SectionHeading>
  <div class="history-filters">
    <Field
      label="Registered from"
      type="date"
      bind:value={from}
      onchange={resetPage}
    />
    <Field
      label="Registered to"
      type="date"
      bind:value={to}
      onchange={resetPage}
    />
    <Button variant="quiet" onclick={clearDates} disabled={!from && !to}
      >Clear dates</Button
    >
  </div>
  <p class="filter-context">
    Filter by registration date. Both dates are included.{" "}{#if from || to}
      Declarations without a registration date are excluded.{:else}
      Declarations without a registration date appear last.{/if}
  </p>
  {#if reversed}<Notice tone="warning" role="alert">
      The start date must be on or before the end date.
    </Notice>
  {:else if filtered.length === 0}<p class="empty-details">
      {details.declarations.length === 0
        ? "No funding history is recorded."
        : "No declarations match these dates. Clear or change the dates."}
    </p>
  {:else}<p class="history-result-count">
      Showing {(page - 1) * pageSize + 1} to {Math.min(
        page * pageSize,
        filtered.length,
      )} of {filtered.length} declarations
    </p>
    <FunderDeclarations declarations={visible} />
    <Pagination
      {page}
      {pageCount}
      onprevious={() => (page -= 1)}
      onnext={() => (page += 1)}
    />
  {/if}
</section>
