<script lang="ts">
  import type { Snippet } from "svelte";
  let {
    heading,
    registered,
    children,
  }: { heading: Snippet; registered: Snippet; children: Snippet } = $props();
</script>

<li class="declaration">
  <header class="declaration-heading">
    <div>{@render heading()}</div>
    <p>{@render registered()}</p>
  </header>
  {@render children()}
</li>
