<script lang="ts">
  let { widths = [60, 44, 72, 53] }: { widths?: number[] } = $props();
</script>

<div class="loading-rows" aria-hidden="true">
  {#each widths as width}<div class="loading-row">
      <span class="skeleton-icon"></span><span
        class="skeleton-name"
        style:width={`${width}%`}
      ></span>
    </div>{/each}
</div>
