<script lang="ts">
  import type { Snippet } from "svelte";
  let {
    children,
    class: className = "entity-kind",
  }: { children: Snippet; class?: string } = $props();
</script>

<span class={className}>{@render children()}</span>
