<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLAnchorAttributes } from "svelte/elements";
  let {
    children,
    ...attributes
  }: HTMLAnchorAttributes & { children: Snippet } = $props();
</script>

<a {...attributes}>{@render children()}</a>
