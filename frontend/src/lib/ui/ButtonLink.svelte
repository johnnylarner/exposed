<script lang="ts">
  import type { HTMLAnchorAttributes } from "svelte/elements";
  import type { ActionVariant, ActionSize } from "./types";
  import "./actions.css";
  let {
    children,
    href,
    variant = "secondary",
    size = "regular",
    class: className = "",
    ...attributes
  }: Omit<HTMLAnchorAttributes, "href"> & {
    href: string;
    variant?: ActionVariant;
    size?: ActionSize;
  } = $props();
</script>

<a
  {...attributes}
  {href}
  class={`ui-action ${className}`}
  data-variant={variant}
  data-size={size}>{@render children?.()}</a
>
