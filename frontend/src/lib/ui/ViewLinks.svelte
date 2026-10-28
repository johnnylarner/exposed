<script lang="ts">
  import type { Snippet } from "svelte";
  import type { ViewLinksAlignment, ViewLinksVariant } from "./types";
  let {
    children,
    label,
    variant = "tabs",
    align = "start",
    wrap = false,
  }: {
    children: Snippet;
    label?: string;
    variant?: ViewLinksVariant;
    align?: ViewLinksAlignment;
    wrap?: boolean;
  } = $props();
</script>

<nav
  class={variant === "filter" ? "view-filters" : "member-tabs"}
  class:category-tabs={wrap}
  class:view-links-end={align === "end"}
  aria-label={label}
>
  {@render children()}
</nav>
