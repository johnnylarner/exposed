<script lang="ts">
  import type { Snippet } from "svelte";
  let {
    children,
    class: className = "",
    variant = "result",
  }: {
    children: Snippet;
    class?: string;
    variant?: "result" | "profile" | "legend";
  } = $props();
</script>

<span
  class={[
    "ui-badge",
    variant === "result" && "entity-kind",
    variant === "profile" && "profile-type",
    variant === "legend" && "legend-item",
    className,
  ]}>{@render children()}</span
>
