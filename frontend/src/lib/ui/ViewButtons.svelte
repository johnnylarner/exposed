<script lang="ts">
  import type { Snippet } from "svelte";
  let { label, children }: { label: string; children: Snippet } = $props();
</script>

<div class="member-tabs category-tabs" role="group" aria-label={label}>
  {@render children()}
</div>
