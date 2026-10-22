<script lang="ts">
  import type { Snippet } from "svelte";
  let { source, value }: { source: Snippet; value: Snippet } = $props();
</script>

<li class="value-row">
  <div class="funding-source">{@render source()}</div>
  <div class="funding-value">{@render value()}</div>
</li>
