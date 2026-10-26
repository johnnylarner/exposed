<script lang="ts">
  import { dev } from "$app/env";
  import { PUBLIC_APP_VERSION } from "$app/env/public";
  import type { Snippet } from "svelte";
  let {
    children,
    target,
    label = "content",
    current,
  }: {
    children: Snippet;
    target: string;
    label?: string;
    current?: string;
  } = $props();
</script>

<a class="skip-link" href={`#${target}`}>Skip to {label}</a>
<div class="app-shell">
  <header class="app-header">
    <a class="wordmark" href="/" aria-label="Exposed home">exposed</a>
    <span class="header-context">UK Parliament</span>
  </header>

  {@render children()}
  <footer class="app-footer">
    <span>Version {PUBLIC_APP_VERSION}</span>
    <a href="/about" aria-current={current === "about" ? "page" : undefined}
      >About</a
    >
    {#if dev}
      <a
        href="/design-system"
        aria-current={current === "design-system" ? "page" : undefined}
        >Design system</a
      >
    {/if}
    <a
      href="https://www.parliament.uk/site-information/copyright-parliament/open-parliament-licence/"
      target="_blank"
      rel="noreferrer">Open Parliament Licence</a
    >
  </footer>
</div>
