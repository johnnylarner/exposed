<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLDetailsAttributes } from "svelte/elements";
  let {
    summary,
    children,
    class: className = "recorded-names",
    label = summary,
    open = $bindable(false),
    role = "region",
    "aria-label": ariaLabel = label,
    ...attributes
  }: Omit<HTMLDetailsAttributes, "children"> & {
    summary: string;
    children: Snippet;
    label?: string;
  } = $props();
</script>

<details
  {...attributes}
  class={className}
  bind:open
  {role}
  aria-label={ariaLabel}
>
  <summary>{summary}</summary>{@render children()}
</details>
