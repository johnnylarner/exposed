<script lang="ts">
  import { categoryBars, registrationDate } from "./lib/entity-details";
  import { formatPounds, type EarningsWeek } from "./lib/member-summary";
  let { weeks, undated }: { weeks: EarningsWeek[]; undated: boolean } =
    $props();
  const bars = $derived(categoryBars(weeks));
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
      (weeks.length > 10 && index % Math.ceil(weeks.length / 4) === 0)
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

<section class="member-earnings" aria-labelledby="earnings-heading">
  <h2 id="earnings-heading">Earnings by registration week</h2>
  {#if weeks.length}
    <svg
      class="earnings-chart"
      viewBox="0 0 560 360"
      role="img"
      aria-labelledby="earnings-title earnings-description"
    >
      <title id="earnings-title">Weekly registered earnings</title><desc
        id="earnings-description"
        >Pound amounts by Monday registration week. Each bar shows its exact
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
      {undated
        ? "Registration dates are unavailable for earnings."
        : "No registered earnings available."}
    </p>{/if}
  {#if undated && weeks.length}<p class="partial-note">
      Earnings without registration dates are excluded.
    </p>{/if}
</section>
