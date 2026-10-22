<script lang="ts">
  import type { HTMLAttributes } from "svelte/elements";
  let {
    children,
    tone = "neutral",
    class: className = "",
    ...attributes
  }: HTMLAttributes<HTMLDivElement> & {
    tone?: "neutral" | "accent" | "warning";
  } = $props();
</script>

<div {...attributes} class={["ui-notice", className]} data-tone={tone}>
  {@render children?.()}
</div>

<style>
  .ui-notice {
    border-left: 2px solid var(--line);
    padding: 12px 16px;
    background: var(--surface);
    color: var(--muted);
    font-size: var(--text-body);
    line-height: 1.8;
  }
  .ui-notice[data-tone="accent"] {
    border-color: var(--accent);
    color: var(--text);
  }
  .ui-notice[data-tone="warning"] {
    border-color: var(--warning);
    color: var(--text);
  }
</style>
