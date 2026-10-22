<script lang="ts">
  import type { Snippet } from "svelte";
  let {
    variant = "default",
    heading,
    registered,
    children,
  }: {
    variant?: "default" | "compact";
    heading: Snippet;
    registered: Snippet;
    children: Snippet;
  } = $props();
</script>

<li class="declaration" data-variant={variant}>
  <header class="declaration-heading">
    <div>{@render heading()}</div>
    <p>{@render registered()}</p>
  </header>
  {@render children()}
</li>
