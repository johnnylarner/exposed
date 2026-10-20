<script lang="ts">
  import { PUBLIC_APP_VERSION } from "$app/env/public";
  import LandingPage from "./LandingPage.svelte";
  import MethodologyPage from "./MethodologyPage.svelte";
  import { pagePresentation } from "./lib/page";
  import SearchPage from "./SearchPage.svelte";
  import EntityDetails from "./EntityDetails.svelte";
  import type { PageModel } from "./lib/page";
  let { model }: { model: PageModel } = $props();
  const presentation = $derived(pagePresentation[model.kind]);
</script>

<svelte:head>
  <title>{presentation.title}</title>
  <meta name="description" content={presentation.description} />
</svelte:head>
<a class="skip-link" href={`#${presentation.target}`}
  >Skip to {presentation.label}</a
>
<div class="app-shell">
  <header class="app-header">
    <a class="wordmark" href="/" aria-label="Exposed home">exposed</a>
    <span class="header-context">UK Parliament</span>
  </header>

  {#if model.kind === "landing"}
    <LandingPage />
  {:else if model.kind === "methodology"}
    <MethodologyPage />
  {:else if model.kind === "search"}
    <SearchPage {model} />
  {:else if model.kind === "detail"}
    <EntityDetails {model} />
  {/if}
  <footer class="app-footer">
    <span>Version {PUBLIC_APP_VERSION}</span>
    <a
      href="/methodology"
      aria-current={model.kind === "methodology" ? "page" : undefined}
      >Methodology</a
    >
    <a
      href="https://www.parliament.uk/site-information/copyright-parliament/open-parliament-licence/"
      target="_blank"
      rel="noreferrer">Open Parliament Licence</a
    >
  </footer>
</div>
