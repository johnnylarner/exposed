<script lang="ts">
  import type { Snippet } from "svelte";
  import type { ViewLinksAlignment, ViewLinksVariant } from "./types";
  let {
    children,
    label,
    variant = "tabs",
    align = "start",
  }: {
    children: Snippet;
    label?: string;
    variant?: ViewLinksVariant;
    align?: ViewLinksAlignment;
  } = $props();
</script>

<nav
  class={variant === "filter" ? "view-filters" : "member-tabs"}
  class:view-links-end={align === "end"}
  aria-label={label}
>
  {@render children()}
</nav>
