<script lang="ts">
  import Methodology from "./methodology.md";
</script>

<main id="methodology" tabindex="-1">
  <article aria-labelledby="methodology-heading">
    <Methodology />
  </article>
</main>

<style>
  main {
    width: min(100%, 74ch);
  }
  main :global(.article-intro) {
    margin-bottom: 32px;
  }
  main :global(h1) {
    font-size: clamp(36px, 5vw, 54px);
    letter-spacing: -1.8px;
    margin-bottom: 20px;
  }
  main :global(p),
  main :global(blockquote) {
    font-size: 15px;
    line-height: 1.85;
  }
  main :global(p) {
    margin: 0 0 18px;
  }
  main :global(.article-intro p) {
    color: var(--muted);
    font-size: 17px;
    line-height: 1.75;
  }
  main :global(a) {
    color: var(--accent);
    text-underline-offset: 4px;
  }
  main :global(a:hover) {
    color: var(--text);
  }
  main :global(section) {
    margin-top: 44px;
  }
  main :global(h2) {
    font-size: 23px;
    font-weight: 600;
    line-height: 1.35;
    letter-spacing: -0.6px;
    margin: 0 0 18px;
  }
  main :global(blockquote) {
    margin: 24px 0;
    padding-left: 20px;
    border-left: 2px solid var(--accent);
  }
  main :global(blockquote p) {
    margin: 0;
  }
  @media (max-width: 480px) {
    main :global(.article-intro p) {
      font-size: 16px;
    }
  }
</style>
