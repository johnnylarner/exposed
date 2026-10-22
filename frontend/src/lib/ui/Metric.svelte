<script lang="ts">
  let {
    label,
    regionLabel = label,
    value,
    available = true,
    description,
    level = 2,
  }: {
    label: string;
    regionLabel?: string;
    value: string;
    available?: boolean;
    description?: string;
    level?: 2 | 3;
  } = $props();
</script>

<div class="member-value">
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
      {value}
    </div>{:else}<p class="member-total unavailable">{value}</p>{/if}
</div>
