<script lang="ts">
  import type { ChartBar } from "./lib/ui";
  import { BarChart } from "./lib/ui";
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
  const chartBars = $derived<ChartBar[]>(
    months.map((month, index) => ({
      x: (index * 560) / months.length + 1,
      y: bars[index].negative
        ? baseline
        : baseline - (bars[index].width / extent) * 314,
      width: Math.max(1, 560 / months.length - 2),
      height: (bars[index].width / extent) * 314,
      negative: bars[index].negative,
      title: `${monthLabel(month.start)}: ${formatPounds(month.amount)}${month.incomplete ? " (some amounts unavailable)" : ""}`,
      tick: tick(index)
        ? {
            x: (index * 540) / Math.max(1, months.length - 1) + 10,
            label: monthLabel(month.start),
            anchor:
              index === 0
                ? "start"
                : index === months.length - 1
                  ? "end"
                  : "middle",
          }
        : undefined,
    })),
  );
</script>

<section class="member-earnings" aria-labelledby={`${context}-heading`}>
  <h2 id={`${context}-heading`}>{copy.heading}</h2>
  {#if months.length}
    <BarChart
      title={copy.title}
      description="Known amounts by UTC registration month. Each bar shows its exact amount in its tooltip."
      {baseline}
      minimum={negative ? formatPounds(minimum) : "£0"}
      minimumY={negative ? 335 : Math.min(335, baseline + 16)}
      bars={chartBars}
    />
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
