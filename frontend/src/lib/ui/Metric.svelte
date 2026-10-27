<script lang="ts">
  import { entryMotion } from "./entry-motion";
  let {
    variant = "default",
    align,
    label,
    regionLabel = label,
    value,
    available = true,
    description,
    level = 2,
    changeKey,
  }: {
    variant?: "default" | "expanded";
    align?: "start" | "end";
    label: string;
    regionLabel?: string;
    value: string;
    available?: boolean;
    description?: string;
    level?: 2 | 3;
    changeKey?: string;
  } = $props();
</script>

<div class="member-value" data-variant={variant} data-align={align}>
  <svelte:element this={`h${level}`} class="ui-metric-label"
    >{label}</svelte:element
  >
  {#if available}<!-- svelte-ignore a11y_no_noninteractive_tabindex (Exact amounts must be keyboard scrollable.) -->
    <div
      class="member-total"
      role="region"
      tabindex="0"
      aria-label={regionLabel}
      title={description}
    >
      <span class="metric-value-clip"
        ><span
          class="metric-value"
          use:entryMotion={{ preset: "roll-up", changeKey }}>{value}</span
        ></span
      >
    </div>{:else}<p class="member-total unavailable">{value}</p>{/if}
</div>
