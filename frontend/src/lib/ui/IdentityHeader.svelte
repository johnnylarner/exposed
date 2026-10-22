<script lang="ts">
  import type { Snippet } from "svelte";
  let {
    name,
    kind,
    metadata,
    values,
    navigation,
    level = 1,
  }: {
    name: string;
    kind: Snippet;
    metadata: Snippet;
    values?: Snippet;
    navigation?: Snippet;
    level?: 1 | 2 | 3;
  } = $props();
</script>

<header class="member-identity">
  <div class="member-identity-info">
    <div class="member-identity-title">
      {@render kind()}
      <svelte:element this={`h${level}`} class="identity-name"
        >{name}</svelte:element
      >
    </div>
    <p>{@render metadata()}</p>
  </div>
  {#if values}{@render values()}{/if}{#if navigation}{@render navigation()}{/if}
</header>
