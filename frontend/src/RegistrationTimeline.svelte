<script lang="ts">
  import { categoryBars } from "./lib/entity-details";
  import { formatPounds } from "./lib/member-summary";
  import type { RegistrationMonth } from "./lib/registration-timeline";
  let {
    months,
    undated,
    context,
  }: {
    months: RegistrationMonth[];
    undated: boolean;
    context: "earnings" | "funding";
  } = $props();
  const copy = $derived(
    {
      earnings: {
        heading: "Earnings by registration month",
        title: "Monthly registered earnings",
        missing: "Registration dates are unavailable for earnings.",
        empty: "No registered earnings available.",
        undated: "Earnings without registration dates are excluded.",
      },
      funding: {
        heading: "Funding by registration month",
        title: "Monthly registered funding",
        missing: "Registration dates are unavailable for funding.",
        empty: "No registered funding available.",
        undated: "Funding without registration dates is excluded.",
      },
    }[context],
  );
  const bars = $derived(categoryBars(months, "maximum"));
  const positive = $derived(
    Math.max(0, ...bars.filter((bar) => !bar.negative).map((bar) => bar.width)),
  );
  const negative = $derived(
    Math.max(0, ...bars.filter((bar) => bar.negative).map((bar) => bar.width)),
  );
  const extent = $derived(positive + negative || 100);
  const baseline = $derived(4 + (positive / extent) * 314);
  const minimum = $derived(
    months[bars.findIndex((bar) => bar.negative && bar.width === negative)]
      ?.amount ?? "0",
  );
  function tick(index: number): boolean {
    return (
      index === 0 ||
      index === months.length - 1 ||
      (months.length > 10 &&
        index % Math.ceil(months.length / 4) === 0 &&
        months.length - 1 - index >= Math.ceil(months.length / 4))
    );
  }
  function monthLabel(start: string): string {
    return new Intl.DateTimeFormat("en-GB", {
      month: "short",
      year: "numeric",
      timeZone: "UTC",
    }).format(new Date(start));
  }
</script>

<section class="member-earnings" aria-labelledby={`${context}-heading`}>
  <h2 id={`${context}-heading`}>{copy.heading}</h2>
  {#if months.length}
    <svg
      class="earnings-chart"
      viewBox="0 0 560 360"
      role="img"
      aria-labelledby={`${context}-title ${context}-description`}
    >
      <title id={`${context}-title`}>{copy.title}</title><desc
        id={`${context}-description`}
        >Known amounts by UTC registration month. Each bar shows its exact
        amount in its tooltip.</desc
      >
      <line
        x1="0"
        x2="560"
        y1={baseline}
        y2={baseline}
        class="chart-baseline"
      />
      {#each months as month, index}{@const bar = bars[index]}{@const height =
          (bar.width / extent) * 314}
        <rect
          x={(index * 560) / months.length + 1}
          y={bar.negative ? baseline : baseline - height}
          width={Math.max(1, 560 / months.length - 2)}
          {height}
          class="earnings-bar"
          class:negative={bar.negative}
          ><title
            >{monthLabel(month.start)}: {formatPounds(
              month.amount,
            )}{month.incomplete ? " (some amounts unavailable)" : ""}</title
          ></rect
        >
        {#if tick(index)}<text
            x={(index * 540) / Math.max(1, months.length - 1) + 10}
            y="354"
            text-anchor={index === 0
              ? "start"
              : index === months.length - 1
                ? "end"
                : "middle"}
            class="chart-axis">{monthLabel(month.start)}</text
          >{/if}{/each}
      <text
        x="0"
        y={negative ? 335 : Math.min(335, baseline + 16)}
        class="chart-axis">{negative ? formatPounds(minimum) : "£0"}</text
      >
    </svg>
    {#if months.some((month) => month.incomplete)}<p class="partial-note">
        Some monthly amounts are unavailable.
      </p>{/if}
  {:else}<p class="empty-details">
      {undated ? copy.missing : copy.empty}
    </p>{/if}
  {#if undated && months.length}<p class="partial-note">
      {copy.undated}
    </p>{/if}
</section>
