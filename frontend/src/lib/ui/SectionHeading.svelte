<script lang="ts">
  import type { Snippet } from "svelte";
  let {
    id,
    title,
    level = 2,
    tone = "neutral",
    children,
  }: {
    id: string;
    title: string;
    level?: 1 | 2 | 3;
    tone?: "neutral" | "accent";
    children?: Snippet;
  } = $props();
</script>

<div class="detail-section-heading" class:accent={tone === "accent"}>
  <svelte:element this={`h${level}`} {id} class="section-title"
    >{title}</svelte:element
  >{#if children}{@render children()}{/if}
</div>

<style>
  .accent {
    background: color-mix(in srgb, var(--accent) 8%, transparent);
    padding: var(--space-2) var(--space-3);
  }
  .accent .section-title {
    margin: 0;
    color: var(--accent);
  }
</style>
