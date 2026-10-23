<script lang="ts">
  import { page } from "$app/state";
  import { DataTable, SectionHeading } from "./lib/ui";
  import { funderHref } from "./lib/entity-details";
  import { formatPounds, type MemberMoneySource } from "./lib/member-summary";

  let { sources }: { sources: MemberMoneySource[] } = $props();
</script>

<section class="money-sources" aria-labelledby="money-sources-heading">
  <SectionHeading id="money-sources-heading" title="Top amount" />
  {#if sources.length}
    <DataTable caption="Sources ranked by known declared GBP amounts">
      {#snippet head()}
        <tr class="visually-hidden"
          ><th scope="col" class="source-name">Source</th><th
            scope="col"
            class="source-value">Declared GBP</th
          ></tr
        >
      {/snippet}
      {#each sources.slice(0, 5) as source}
        <tr>
          <th scope="row" class="source-name">
            {#if source.funder}<a
                href={funderHref(
                  source.funder.id,
                  "summary",
                  page.url.searchParams,
                )}>{source.funder.name}</a
              >{:else}Funder unavailable{/if}
          </th>
          <td class="source-value">
            {#if source.amount === null}<span class="partial-note"
                >{formatPounds(source.amount)}</span
              >{:else}<!-- svelte-ignore a11y_no_noninteractive_tabindex (Exact amounts must remain keyboard-scrollable.) -->
              <div
                class="source-amount"
                role="region"
                aria-label={`Known declared GBP from ${source.funder?.name ?? "unavailable source"}`}
                tabindex="0"
              >
                <span class="amount">{formatPounds(source.amount)}</span>
              </div>{/if}
            {#if source.amount !== null && (source.missingGbpAmountCount > 0 || source.unknownCurrencyCount > 0)}
              <span class="partial-note">Some amounts unavailable</span>
            {/if}
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
