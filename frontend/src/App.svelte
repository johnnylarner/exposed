<script lang="ts">
  import AppShell from "./AppShell.svelte";
  import SearchPage from "./SearchPage.svelte";
  import EntityDetails from "./EntityDetails.svelte";
  import type { PageModel } from "./lib/page";
  let { model }: { model: PageModel } = $props();
</script>

<AppShell mainTarget={model.kind === "search" ? "search" : "details"}>
  {#if model.kind === "search"}<SearchPage {model} />{:else}<EntityDetails
      {model}
    />{/if}
</AppShell>
