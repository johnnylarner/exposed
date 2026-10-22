<script lang="ts">
  import type { Snippet } from "svelte";
  let {
    id,
    title,
    level = 2,
    children,
  }: {
    id: string;
    title: string;
    level?: 1 | 2 | 3;
    children?: Snippet;
  } = $props();
</script>

<div class="detail-section-heading">
  <svelte:element this={`h${level}`} {id} class="section-title"
    >{title}</svelte:element
  >{#if children}{@render children()}{/if}
</div>
