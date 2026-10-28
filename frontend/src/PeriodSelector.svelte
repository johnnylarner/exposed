<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { Select } from "./lib/ui";
  import { periodOptions, type ReportingPeriod } from "./lib/reporting-period";
  let { period }: { period: ReportingPeriod } = $props();
  function href(selected: ReportingPeriod) {
    const url = new URL(page.url.href);
    url.searchParams.set("period", selected);
    for (const key of ["from", "to", "page"]) url.searchParams.delete(key);
    return url.pathname + url.search;
  }
</script>

<Select
  label="Reporting period"
  options={periodOptions}
  value={period}
  onselect={(selected) => {
    void goto(href(selected), { reset: false });
  }}
/>
