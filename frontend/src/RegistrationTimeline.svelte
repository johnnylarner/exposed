<script lang="ts">
  import { categoryBars, registrationDate } from "./lib/entity-details";
  import { formatPounds } from "./lib/member-summary";
  import type { RegistrationWeek } from "./lib/registration-timeline";
  let {
    weeks,
    undated,
    context,
  }: {
    weeks: RegistrationWeek[];
    undated: boolean;
    context: "earnings" | "funding";
  } = $props();
  const copy = $derived(
    {
      earnings: {
        heading: "Earnings by registration week",
        title: "Weekly registered earnings",
        missing: "Registration dates are unavailable for earnings.",
        empty: "No registered earnings available.",
        undated: "Earnings without registration dates are excluded.",
      },
      funding: {
        heading: "Funding by registration week",
        title: "Weekly registered funding",
        missing: "Registration dates are unavailable for funding.",
        empty: "No registered funding available.",
        undated: "Funding without registration dates is excluded.",
      },
    }[context],
  );
  const bars = $derived(categoryBars(weeks, "maximum"));
  const positive = $derived(
    Math.max(0, ...bars.filter((bar) => !bar.negative).map((bar) => bar.width)),
  );
  const negative = $derived(
    Math.max(0, ...bars.filter((bar) => bar.negative).map((bar) => bar.width)),
  );
  const extent = $derived(positive + negative || 100);
  const baseline = $derived(4 + (positive / extent) * 314);
  const minimum = $derived(
    weeks[bars.findIndex((bar) => bar.negative && bar.width === negative)]
      ?.amount ?? "0",
  );
  function tick(index: number): boolean {
    return (
      index === 0 ||
      index === weeks.length - 1 ||
      (weeks.length > 10 &&
        index % Math.ceil(weeks.length / 4) === 0 &&
        weeks.length - 1 - index >= Math.ceil(weeks.length / 4))
    );
  }
  function month(start: string): string {
    return new Intl.DateTimeFormat("en-GB", {
      month: "short",
      year: "numeric",
      timeZone: "UTC",
    }).format(new Date(start));
  }
</script>

<section class="member-earnings" aria-labelledby={`${context}-heading`}>
  <h2 id={`${context}-heading`}>{copy.heading}</h2>
  {#if weeks.length}
    <svg
      class="earnings-chart"
      viewBox="0 0 560 360"
      role="img"
      aria-labelledby={`${context}-title ${context}-description`}
    >
      <title id={`${context}-title`}>{copy.title}</title><desc
        id={`${context}-description`}
        >Known amounts by Monday registration week. Each bar shows its exact
        amount in its tooltip.</desc
      >
      <line
        x1="0"
        x2="560"
        y1={baseline}
        y2={baseline}
        class="chart-baseline"
      />
      {#each weeks as week, index}{@const bar = bars[index]}{@const height =
          (bar.width / extent) * 314}
        <rect
          x={(index * 560) / weeks.length + 1}
          y={bar.negative ? baseline : baseline - height}
          width={Math.max(1, 560 / weeks.length - 2)}
          {height}
          class="earnings-bar"
          class:negative={bar.negative}
          ><title
            >Week of {registrationDate(week.start)}: {formatPounds(
              week.amount,
            )}{week.incomplete ? " (some amounts unavailable)" : ""}</title
          ></rect
        >
        {#if tick(index)}<text
            x={(index * 540) / Math.max(1, weeks.length - 1) + 10}
            y="354"
            text-anchor={index === 0
              ? "start"
              : index === weeks.length - 1
                ? "end"
                : "middle"}
            class="chart-axis">{month(week.start)}</text
          >{/if}{/each}
      <text
        x="0"
        y={negative ? 335 : Math.min(335, baseline + 16)}
        class="chart-axis">{negative ? formatPounds(minimum) : "£0"}</text
      >
    </svg>
    {#if weeks.some((week) => week.incomplete)}<p class="partial-note">
        Some weekly amounts are unavailable.
      </p>{/if}
  {:else}<p class="empty-details">
      {undated ? copy.missing : copy.empty}
    </p>{/if}
  {#if undated && weeks.length}<p class="partial-note">
      {copy.undated}
    </p>{/if}
</section>
