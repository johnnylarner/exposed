<script lang="ts">
  import { Metric } from "./lib/ui";
  import { formatPounds } from "./lib/member-summary";
  import type { DeclaredGbpTotals } from "./lib/entity-details";
  import { declaredTotal, type ReportingPeriod } from "./lib/reporting-period";
  let {
    totals,
    period,
  }: { totals: DeclaredGbpTotals; period: ReportingPeriod } = $props();
  const total = $derived(declaredTotal(totals, period));
</script>

<div class="declared-totals">
  <Metric
    variant="expanded"
    label="Declared amounts"
    regionLabel="Declared amounts in GBP"
    value={total.declarationCount === 0
      ? "No declarations recorded"
      : formatPounds(total.amount)}
    available={total.declarationCount > 0 && total.amount !== null}
    description={total.incomplete
      ? "Known GBP amounts only. Some amounts unavailable."
      : undefined}
  />
</div>
