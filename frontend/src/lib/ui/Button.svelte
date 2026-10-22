<script lang="ts">
  import type { HTMLButtonAttributes } from "svelte/elements";
  import type { ActionVariant, ActionSize } from "./types";
  let {
    children,
    variant = "secondary",
    size = "regular",
    type = "button",
    class: className = "",
    ...attributes
  }: HTMLButtonAttributes & {
    variant?: ActionVariant;
    size?: ActionSize;
  } = $props();
</script>

<button
  {...attributes}
  {type}
  class={["ui-action", className]}
  data-variant={variant}
  data-size={size}>{@render children?.()}</button
>
