<script lang="ts">
  import type { Snippet } from "svelte";
  let { children, label }: { children: Snippet; label?: string } = $props();
</script>

<li class="entity-row" aria-label={label}>{@render children()}</li>
