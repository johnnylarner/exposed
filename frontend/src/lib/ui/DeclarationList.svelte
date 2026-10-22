<script lang="ts">
  import type { Snippet } from "svelte";
  let { children, label }: { children: Snippet; label?: string } = $props();
</script>

<ol class="declarations-list" aria-label={label}>
  {@render children()}
</ol>
