<script lang="ts">
  import type { Snippet } from "svelte";
  let {
    children,
    label,
    class: className = "",
  }: { children: Snippet; label?: string; class?: string } = $props();
</script>

<li class={["entity-row", className]} aria-label={label}>
  {@render children()}
</li>
