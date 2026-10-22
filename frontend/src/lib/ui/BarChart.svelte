<script lang="ts">
  import type { ChartBar } from "./chart-types";
  let {
    title,
    description,
    bars,
    baseline,
    minimum,
    minimumY,
  }: {
    title: string;
    description: string;
    bars: ChartBar[];
    baseline: number;
    minimum: string;
    minimumY: number;
  } = $props();
  const id = $props.id();
</script>

<svg
  class="earnings-chart"
  viewBox="0 0 560 360"
  role="img"
  aria-labelledby={`${id}-title ${id}-description`}
  ><title id={`${id}-title`}>{title}</title><desc id={`${id}-description`}
    >{description}</desc
  ><line
    x1="0"
    x2="560"
    y1={baseline}
    y2={baseline}
    class="chart-baseline"
  />{#each bars as bar}<rect
      x={bar.x}
      y={bar.y}
      width={bar.width}
      height={bar.height}
      class="earnings-bar"
      class:negative={bar.negative}><title>{bar.title}</title></rect
    >{#if bar.tick}<text
        x={bar.tick.x}
        y="354"
        text-anchor={bar.tick.anchor}
        class="chart-axis">{bar.tick.label}</text
      >{/if}{/each}<text x="0" y={minimumY} class="chart-axis">{minimum}</text
  ></svg
>
