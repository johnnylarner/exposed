<script lang="ts">
  import PeriodSelector from "./PeriodSelector.svelte";
  import DeclaredTotals from "./DeclaredTotals.svelte";
  import type { PieSlice } from "./lib/ui";
  import {
    ContentChange,
    PieChart,
    DataTable,
    Disclosure,
    Notice,
    SectionHeading,
  } from "./lib/ui";
  import { page } from "$app/state";

  import { formatPounds } from "./lib/member-summary";
  import { memberHref, type FunderDetails } from "./lib/entity-details";
  import { partyChart } from "./lib/funder-summary";
  let { details }: { details: FunderDetails } = $props();

  const pounds = $derived(
    details.currencies.find((group) => group.currency === "GBP"),
  );
  const slices = $derived(partyChart(pounds?.parties ?? []));
  const recipients = $derived(pounds?.topRecipients.slice(0, 5) ?? []);
  const chartSlices = $derived<PieSlice[]>(
    slices.map((slice) => ({
      share: slice.share,
      path: slice.path,
      color: slice.color,
      label: slice.party.partyName || "Party unavailable",
      value: formatPounds(slice.party.amount),
      detail: `${slice.party.entryCount} ${slice.party.entryCount === 1 ? "entry" : "entries"}${slice.party.unknownAmountCount > 0 ? `, ${slice.party.unknownAmountCount} ${slice.party.unknownAmountCount === 1 ? "amount" : "amounts"} unavailable` : ""}`,
    })),
  );
</script>

<section aria-label="Summary">
  <div class="summary-header">
    <DeclaredTotals totals={details.declaredTotals} period={details.period} />
    <PeriodSelector period={details.period} />
  </div>
  <ContentChange changeKey={details.period}>
    <p class="entry-count">
      {details.entryCount}
      {details.entryCount === 1 ? "funding entry" : "funding entries"}
    </p>
    <div class="member-summary-top funder-summary-top">
      <section aria-labelledby="party-heading">
        <SectionHeading id="party-heading" title="Funding by party" />
        <div class="funder-party-chart">
          {#if slices.length === 0}<p class="empty-details funder-party-empty">
              {details.entryCount === 0
                ? "No funding entries are recorded in this period."
                : "No known funding totals are available."}
            </p>{:else}
            <PieChart
              label="Funding by party. Positive totals determine the pie shares."
              slices={chartSlices}
            />
          {/if}
        </div>
        {#if slices.length > 0}
          {#if slices.every((slice) => slice.share === 0)}<Notice>
              Pie unavailable. No positive known totals are recorded.
            </Notice>{:else if slices.some((slice) => slice.party.amount === null || slice.party.amount.startsWith("-"))}<p
              class="filter-context"
            >
              The pie shows positive known totals. Negative and unavailable
              totals are available in the party legend.
            </p>{/if}
        {/if}
      </section>
      <section
        class="funder-recipient-panel"
        aria-labelledby="recipients-heading"
      >
        <SectionHeading id="recipients-heading" title="Top 5 recipients" />
        {#if !recipients.length}<p class="empty-details">
            No recipients are available.
          </p>{:else}
          <div class="funder-recipients">
            <DataTable caption="Top recipients of declared funding"
              >{#snippet head()}<tr class="visually-hidden"
                  ><th class="recipient-name" scope="col">MP</th><th
                    class="recipient-total"
                    scope="col">Known total</th
                  ></tr
                >{/snippet}
              {#each recipients as recipient}<tr
                  ><th class="recipient-name" scope="row"
                    ><a
                      href={memberHref(
                        recipient.member.id,
                        "summary",
                        page.url.searchParams,
                      )}>{recipient.member.name}</a
                    ><span class="entry-count"
                      >{recipient.member.partyName || "Party unavailable"}</span
                    ></th
                  ><td class="recipient-total"
                    ><span class="amount">{formatPounds(recipient.amount)}</span
                    >{#if recipient.unknownAmountCount > 0}<span
                        class="partial-note"
                        >{recipient.unknownAmountCount}
                        {recipient.unknownAmountCount === 1
                          ? "amount"
                          : "amounts"} unavailable</span
                      >{/if}</td
                  ></tr
                >{/each}
            </DataTable>
          </div>
        {/if}
      </section>
    </div>
  </ContentChange>
  {#if details.funder.aliases.length > 0}<Disclosure
      summary="Recorded names"
      class="funder-recorded-names"
    >
      <ul>
        {#each details.funder.aliases as alias}<li>{alias}</li>{/each}
      </ul>
    </Disclosure>{/if}
</section>

<style>
  th.recipient-name {
    width: 54%;
  }
  th[scope="col"] {
    padding-right: 16px;
  }
  th.recipient-total,
  td.recipient-total {
    text-align: right;
  }
  .recipient-name a {
    text-decoration-color: #437c6e;
    text-underline-offset: 4px;
  }
</style>
