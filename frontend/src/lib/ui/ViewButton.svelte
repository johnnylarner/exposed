<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";
  let {
    children,
    selected,
    ...attributes
  }: Omit<HTMLButtonAttributes, "aria-pressed" | "children"> & {
    children: Snippet;
    selected: boolean;
  } = $props();
</script>

<button type="button" {...attributes} aria-pressed={selected}
  >{@render children()}</button
>
