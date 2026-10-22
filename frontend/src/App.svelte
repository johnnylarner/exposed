<script lang="ts">
  import AppShell from "./AppShell.svelte";
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
<AppShell
  target={presentation.target}
  label={presentation.label}
  current={model.kind}
>
  {#if model.kind === "landing"}
    <LandingPage />
  {:else if model.kind === "methodology"}
    <MethodologyPage />
  {:else if model.kind === "search"}
    <SearchPage {model} />
  {:else if model.kind === "detail"}
    <EntityDetails {model} />
  {/if}
</AppShell>
