<script lang="ts">
  import type { Snippet } from "svelte";
  import { entryMotion } from "./entry-motion";
  let { changeKey, children }: { changeKey: string; children: Snippet } =
    $props();
</script>

<div class="content-change" use:entryMotion={{ preset: "fade", changeKey }}>
  {@render children()}
</div>
