<script lang="ts">
  import { entryMotion } from "./entry-motion";
  import type { SignedBar } from "./chart-types";
  let {
    bars,
    baseline,
    changeKey,
    variant = "default",
  }: {
    bars: SignedBar[];
    baseline: number;
    changeKey?: string;
    variant?: "default" | "compact";
  } = $props();
</script>

<ul
  class="category-list"
  data-variant={variant}
  style:--baseline={`${baseline}%`}
>
  {#each bars as bar, index}<li
      use:entryMotion={{ preset: "row", changeKey, initial: true, index }}
    >
      <div class="category-label">
        <span>{bar.label}</span><span class="amount">{bar.value}</span>
      </div>
      <div class="signed-bar" aria-hidden="true">
        <span
          use:entryMotion={{ preset: "bar", changeKey, initial: true, index }}
          class:negative={bar.negative}
          style:width={`${bar.width}%`}
        ></span>
      </div>
      {#if bar.note}<p class="partial-note">{bar.note}</p>{/if}
    </li>{/each}
</ul>
