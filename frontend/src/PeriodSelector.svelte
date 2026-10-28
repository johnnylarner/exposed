<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { ViewLinks, ViewLink, Select } from "./lib/ui";
  import { periodOptions, type ReportingPeriod } from "./lib/reporting-period";
  let {
    period,
    variant = "links",
  }: { period: ReportingPeriod; variant?: "links" | "dropdown" } = $props();
  function href(selected: ReportingPeriod) {
    const url = new URL(page.url.href);
    url.searchParams.set("period", selected);
    for (const key of ["from", "to", "page"]) url.searchParams.delete(key);
    return url.pathname + url.search;
  }
</script>

{#if variant === "dropdown"}
  <Select
    label="Reporting period"
    options={periodOptions}
    value={period}
    onselect={(selected) => {
      void goto(href(selected), { reset: false });
    }}
  />
{:else}
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
{/if}
