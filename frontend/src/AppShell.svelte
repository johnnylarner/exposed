<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    mainTarget,
    children,
  }: {
    mainTarget: "search" | "details" | "methodology";
    children: Snippet;
  } = $props();
</script>

<a class="skip-link" href={`#${mainTarget}`}>Skip to {mainTarget}</a>
<div class="app-shell">
  <header class="app-header">
    <a class="wordmark" href="/" aria-label="Exposed home">exposed</a>
    <span class="header-context">UK Parliament</span>
  </header>

  {@render children()}

  <footer class="app-footer">
    <span>Contains Parliamentary information.</span>
    <a
      href="/methodology"
      aria-current={mainTarget === "methodology" ? "page" : undefined}
      >Methodology</a
    >
    <a
      href="https://www.parliament.uk/site-information/copyright-parliament/open-parliament-licence/"
      target="_blank"
      rel="noreferrer">Open Parliament Licence</a
    >
  </footer>
</div>
