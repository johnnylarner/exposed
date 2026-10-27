<script lang="ts">
  import PeriodSelector from "./PeriodSelector.svelte";
  import DeclaredTotals from "./DeclaredTotals.svelte";
  import type { ReportingPeriod } from "./lib/reporting-period";
  import { ContentChange, SignedBarList } from "./lib/ui";
  import MemberMoneySources from "./MemberMoneySources.svelte";
  import { categoryBars } from "./lib/entity-details";
  import { formatPounds, summarizeMember } from "./lib/member-summary";
  let {
    summary,
    period,
  }: { summary: ReturnType<typeof summarizeMember>; period: ReportingPeriod } =
    $props();
  const bars = $derived(categoryBars(summary.categories, "total"));
  const baseline = $derived(
    bars.reduce((total, bar) => total + (bar.negative ? bar.width : 0), 0),
  );
</script>

<section aria-label="Summary">
  <PeriodSelector {period} />
  <ContentChange changeKey={period}>
    <DeclaredTotals totals={summary.declaredTotals} {period} />
    <div class="member-summary-top">
      <section class="member-breakdown" aria-labelledby="breakdown-heading">
        <h2 class="section-title" id="breakdown-heading">
          Declared value by category
        </h2>
        {#if summary.categories.length}
          <SignedBarList
            variant="compact"
            {baseline}
            bars={summary.categories.map((category, index) => ({
              label: category.categoryName,
              value: formatPounds(category.amount),
              width: bars[index].width,
              negative: bars[index].negative,
              note: category.unknownAmountCount
                ? "Some amounts unavailable"
                : undefined,
            }))}
          />
        {:else}<p class="empty-details">
            No known declared values available.
          </p>{/if}
      </section>
      <MemberMoneySources sources={summary.sources} />
    </div>
  </ContentChange>
</section>
