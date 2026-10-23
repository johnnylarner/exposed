<script lang="ts">
  import { page } from "$app/state";
  import { DataTable, SectionHeading } from "./lib/ui";
  import { funderHref } from "./lib/entity-details";
  import { formatPounds, type MemberMoneySource } from "./lib/member-summary";

  let { sources }: { sources: MemberMoneySource[] } = $props();
</script>

<section class="money-sources" aria-labelledby="money-sources-heading">
  <SectionHeading
    id="money-sources-heading"
    title="Declared amounts by source"
  />
  {#if sources.length}
    <DataTable caption="Sources ranked by known declared GBP amounts">
      {#snippet head()}
        <tr
          ><th scope="col" class="source-name">Source</th><th
            scope="col"
            class="source-value">Known declared GBP</th
          ></tr
        >
      {/snippet}
      {#each sources as source}
        <tr>
          <th scope="row" class="source-name">
            {#if source.funder}<a
                href={funderHref(
                  source.funder.id,
                  "summary",
                  page.url.searchParams,
                )}>{source.funder.name}</a
              >{:else}Source unavailable{/if}
            <span class="entry-count"
              >{source.entryCount}
              {source.entryCount === 1 ? "entry" : "entries"}</span
            >
          </th>
          <td class="source-value">
            {#if source.amount === null}<span class="partial-note"
                >No known GBP amount</span
              >{:else}<!-- svelte-ignore a11y_no_noninteractive_tabindex (Exact amounts must remain keyboard-scrollable.) -->
              <div
                class="source-amount"
                role="region"
                aria-label={`Known declared GBP from ${source.funder?.name ?? "unavailable source"}`}
                tabindex="0"
              >
                <span class="amount">{formatPounds(source.amount)}</span>
              </div>{/if}
            {#if source.missingGbpAmountCount}<span class="partial-note"
                >{source.missingGbpAmountCount} GBP {source.missingGbpAmountCount ===
                1
                  ? "amount"
                  : "amounts"} unavailable</span
              >{/if}
            {#if source.unknownCurrencyCount}<span class="partial-note"
                >{source.unknownCurrencyCount}
                {source.unknownCurrencyCount === 1 ? "entry" : "entries"} with unknown
                currency excluded</span
              >{/if}
            {#if source.foreignCurrencyCount}<span class="partial-note"
                >{source.foreignCurrencyCount} foreign-currency {source.foreignCurrencyCount ===
                1
                  ? "entry"
                  : "entries"} excluded</span
              >{/if}
          </td>
        </tr>
      {/each}
    </DataTable>
  {:else}<p class="empty-details">
      No funding entries are stored for this MP.
    </p>{/if}
</section>

<style>
  .money-sources {
    min-width: 0;
  }
  .source-name {
    width: 48%;
    padding-right: var(--space-4);
  }
  .source-value {
    text-align: right;
  }
  .source-amount {
    max-width: 100%;
    overflow-x: auto;
    white-space: nowrap;
  }
</style>
