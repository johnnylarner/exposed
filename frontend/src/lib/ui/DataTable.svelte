<script lang="ts">
  import type { Snippet } from "svelte";
  let {
    caption,
    head,
    children,
  }: { caption: string; head: Snippet; children: Snippet } = $props();
</script>

<table class="ui-data-table">
  <caption class="visually-hidden">{caption}</caption><thead
    >{@render head()}</thead
  ><tbody>{@render children()}</tbody>
</table>
