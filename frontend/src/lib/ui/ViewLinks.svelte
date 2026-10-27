<script lang="ts">
  import type { Snippet } from "svelte";
  import type { ViewLinksVariant } from "./types";
  let {
    children,
    label,
    variant = "tabs",
  }: {
    children: Snippet;
    label?: string;
    variant?: ViewLinksVariant;
  } = $props();
</script>

<nav
  class={variant === "filter" ? "view-filters" : "member-tabs"}
  aria-label={label}
>
  {@render children()}
</nav>
