<script lang="ts">
  import { page } from "$app/state";
  import { ViewLinks, ViewLink } from "./lib/ui";
  import { periodOptions, type ReportingPeriod } from "./lib/reporting-period";
  let { period }: { period: ReportingPeriod } = $props();
  function href(selected: ReportingPeriod) {
    const url = new URL(page.url.href);
    url.searchParams.set("period", selected);
    for (const key of ["from", "to", "page"]) url.searchParams.delete(key);
    return url.pathname + url.search;
  }
</script>

<ViewLinks variant="filter" align="end" label="Reporting period">
  {#each periodOptions as option}
    <ViewLink
      href={href(option.value)}
      data-sveltekit-reset={false}
      aria-current={period === option.value ? "page" : undefined}
      >{option.label}</ViewLink
    >
  {/each}
</ViewLinks>
