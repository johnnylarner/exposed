<script lang="ts">
  import { Metric } from "./lib/ui";
  import { formatPounds } from "./lib/member-summary";
  import type { DeclaredGbpTotals } from "./lib/entity-details";
  let {
    totals,
    firstRegisteredAt = null,
  }: {
    totals: DeclaredGbpTotals;
    firstRegisteredAt?: string | null;
  } = $props();
  function dateLabel(value: string) {
    return new Date(value).toLocaleDateString("en-GB", {
      day: "numeric",
      month: "long",
      year: "numeric",
      timeZone: "UTC",
    });
  }
</script>

<div class="declared-totals">
  <div class="declared-values">
    {#each [{ label: "Declared amounts this parliamentary term", regionLabel: "Declared amounts this parliamentary term in GBP", total: totals.currentTerm }, { regionLabel: "Historical declared amounts in GBP", label: `Historical declared amounts${firstRegisteredAt ? ` since ${dateLabel(firstRegisteredAt)}` : ""}`, total: totals.historical }, { label: `Declared amounts in ${totals.calendarYear}`, regionLabel: `Declared amounts in ${totals.calendarYear} in GBP`, total: totals.currentYear }] as metric}<Metric
        variant="expanded"
        regionLabel={metric.regionLabel}
        label={metric.label}
        value={metric.total.declarationCount === 0
          ? "No declarations recorded"
          : formatPounds(metric.total.amount)}
        available={metric.total.declarationCount > 0 &&
          metric.total.amount !== null}
        description={metric.total.incomplete
          ? "Known GBP amounts only. Some amounts unavailable."
          : undefined}
      />{/each}
  </div>
</div>
