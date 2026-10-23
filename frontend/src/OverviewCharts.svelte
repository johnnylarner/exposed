<script lang="ts">
  import { BarChart, PieChart, SignedBarList } from "./lib/ui";
  import { overviewCharts, type OverviewState } from "./lib/overview";
  import { formatPounds } from "./lib/member-summary";
  let { state }: { state: OverviewState } = $props();
  const charts = $derived(
    state.kind === "available" ? overviewCharts(state.overview) : null,
  );
</script>

<section class="overview" aria-labelledby="overview-heading">
  <h2 id="overview-heading">Declared GBP amounts</h2>
  {#if state.kind === "available" && charts}
    {@const overview = state.overview}
    <p class="coverage">
      {overview.memberCount} current Commons MPs · Since {new Intl.DateTimeFormat(
        "en-GB",
        { day: "numeric", month: "long", year: "numeric", timeZone: "UTC" },
      ).format(new Date(overview.currentTermStart))}{overview.incomplete
        ? " · Partial"
        : ""}{overview.unavailableMemberCount
        ? ` · ${overview.unavailableMemberCount} amounts unavailable`
        : ""}
    </p>
    <div class="chart-grid">
      <section class="distribution" aria-labelledby="quartiles-heading">
        <h3 id="quartiles-heading">Distribution by quartile</h3>
        {#if overview.quartiles.some((q) => q.memberCount)}
          <BarChart
            title="Declared GBP ranges by rank quartile"
            description="Current Commons MPs ranked by known declared GBP amount, lowest to highest. Bars show each quartile’s minimum and maximum."
            bars={charts.quartiles}
            baseline={charts.baseline}
            minimum={charts.minimum}
            minimumY={320}
          />
          <ul class="quartile-values">
            {#each overview.quartiles as q}<li>
                <span>Q{q.quartile} · {q.memberCount} MPs</span><span
                  >{q.memberCount
                    ? `${formatPounds(q.minimum)} to ${formatPounds(q.maximum)}`
                    : "Unavailable"}{q.incompleteMemberCount
                    ? " · Partial"
                    : ""}</span
                >
              </li>{/each}
          </ul>
        {:else}<p class="coverage">No known declared values available.</p>{/if}
      </section>
      <section aria-labelledby="top-heading">
        <h3 id="top-heading">Top 5 MPs by declared amount</h3>
        {#if charts.members.length}
          <SignedBarList
            bars={charts.members}
            baseline={charts.memberBaseline}
            variant="compact"
          />
        {:else}<p class="coverage">No known declared values available.</p>{/if}
      </section>
      <section class="party-chart" aria-labelledby="party-heading">
        <h3 id="party-heading">Declared amount by party</h3>
        {#if charts.negativeParty}<p class="coverage">
            Pie chart unavailable for negative declared amounts.
          </p>
          <ul class="party-values">
            {#each overview.parties as row}<li>
                {row.partyName} · {formatPounds(row.total.amount)}{row.total
                  .incomplete
                  ? " · Partial"
                  : ""}
              </li>{/each}
          </ul>
        {:else if charts.parties.some((party) => party.share > 0)}<PieChart
            legendLayout="list"
            slices={charts.parties}
            label="Declared GBP amounts by latest recorded party"
          />
        {:else}<p class="coverage">No positive declared values available.</p>
          <ul class="party-values">
            {#each overview.parties as row}<li>
                {row.partyName} · {formatPounds(row.total.amount)}
              </li>{/each}
          </ul>
        {/if}
      </section>
    </div>
  {:else}<p class="coverage">Declared amounts are unavailable.</p>{/if}
</section>

<style>
  .overview {
    width: 100%;
    margin-top: 64px;
    border-top: 1px solid var(--line);
    padding-top: 32px;
  }
  h2 {
    margin: 0;
    font-size: var(--text-heading);
    font-weight: 550;
  }
  h3 {
    margin: 0 0 24px;
    font-size: 1rem;
    font-weight: 550;
  }
  .coverage {
    color: var(--muted);
    font-size: var(--text-body);
    margin: 12px 0 32px;
  }
  .chart-grid {
    display: grid;
    grid-template-columns: 1.1fr 1fr 1fr;
    gap: 40px;
    align-items: start;
  }
  .chart-grid > section {
    min-width: 0;
  }
  .quartile-values,
  .party-values {
    list-style: none;
    padding: 0;
    margin: 16px 0 0;
    font-size: var(--text-small);
  }
  .quartile-values li {
    display: grid;
    gap: 4px;
    margin-top: 12px;
  }
  .quartile-values li span:first-child {
    color: var(--muted);
  }
  .party-values li {
    margin-top: 12px;
    overflow-wrap: anywhere;
  }
  @media (max-width: 1000px) {
    .chart-grid {
      grid-template-columns: 1fr 1fr;
    }
    .party-chart {
      grid-column: 1 / -1;
    }
  }
  @media (max-width: 600px) {
    .chart-grid {
      grid-template-columns: 1fr;
      gap: 40px;
    }
    .party-chart {
      grid-column: auto;
    }
    .overview {
      margin-top: 48px;
    }
  }
</style>
