<script lang="ts">
  import SearchPage from "./SearchPage.svelte";
  import EntityDetails from "./EntityDetails.svelte";
  import { readDetailRoute } from "./lib/entity-details";
  const route = readDetailRoute(window.location.pathname);
</script>

<a class="skip-link" href={route.kind === "search" ? "#search" : "#details"}
  >Skip to {route.kind === "search" ? "search" : "details"}</a
>
<div class="app-shell">
  <header class="app-header">
    <a class="wordmark" href="/" aria-label="Exposed home">exposed</a>
    <span class="header-context">UK Parliament</span>
  </header>

  {#if route.kind === "search"}<SearchPage />{:else}<EntityDetails
      {route}
    />{/if}
  <footer class="app-footer">
    <span>Contains Parliamentary information.</span><a
      href="https://www.parliament.uk/site-information/copyright-parliament/open-parliament-licence/"
      target="_blank"
      rel="noreferrer">Open Parliament Licence</a
    >
  </footer>
</div>
