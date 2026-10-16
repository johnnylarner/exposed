<script lang="ts">
  import SearchPage from "./SearchPage.svelte";
  import EntityDetails from "./EntityDetails.svelte";
  import type { PageModel } from "./lib/page";
  let { model }: { model: PageModel } = $props();
</script>

<a class="skip-link" href={model.kind === "search" ? "#search" : "#details"}
  >Skip to {model.kind === "search" ? "search" : "details"}</a
>
<div class="app-shell">
  <header class="app-header">
    <a class="wordmark" href="/" aria-label="Exposed home">exposed</a>
    <span class="header-context">UK Parliament</span>
  </header>

  {#if model.kind === "search"}<SearchPage {model} />{:else}<EntityDetails
      {model}
    />{/if}
  <footer class="app-footer">
    <span>Contains Parliamentary information.</span><a
      href="https://www.parliament.uk/site-information/copyright-parliament/open-parliament-licence/"
      target="_blank"
      rel="noreferrer">Open Parliament Licence</a
    >
  </footer>
</div>
