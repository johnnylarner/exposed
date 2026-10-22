<script lang="ts">
  import type { SignedBar } from "./chart-types";
  let {
    bars,
    baseline,
    variant = "default",
  }: {
    bars: SignedBar[];
    baseline: number;
    variant?: "default" | "compact";
  } = $props();
</script>

<ul
  class="category-list"
  data-variant={variant}
  style:--baseline={`${baseline}%`}
>
  {#each bars as bar}<li>
      <div class="category-label">
        <span>{bar.label}</span><span class="amount">{bar.value}</span>
      </div>
      <div class="signed-bar" aria-hidden="true">
        <span class:negative={bar.negative} style:width={`${bar.width}%`}
        ></span>
      </div>
      {#if bar.note}<p class="partial-note">{bar.note}</p>{/if}
    </li>{/each}
</ul>
