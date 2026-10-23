<script lang="ts">
  import OverviewCharts from "./OverviewCharts.svelte";
  import type { OverviewState } from "./lib/overview";
  let { overview }: { overview: OverviewState } = $props();
  import { ButtonLink } from "./lib/ui";
</script>

<main id="landing" class="landing-page" tabindex="-1">
  <div class="landing-content">
    <h1>
      Explore <span class="member-text">MPs</span>' declared
      <span class="funder-text">financial interests</span> and the people and organisations
      behind them.
    </h1>
    <nav aria-label="Explore Exposed">
      <ButtonLink variant="primary" size="large" href="/search"
        >Search MPs and funders</ButtonLink
      >
      <ButtonLink size="large" href="/methodology">Methodology</ButtonLink>
    </nav>
  </div>
  <OverviewCharts state={overview} />
</main>

<style>
  .landing-page {
    width: 100%;
    margin: 0;
    padding: 64px 0 80px;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
  }
  .landing-content {
    max-width: 1050px;
  }
  h1 {
    margin: 0;
    max-width: 19ch;
    font-size: clamp(36px, 4.8vw, 64px);
    line-height: 1.2;
    letter-spacing: -0.045em;
    font-weight: 550;
    text-wrap: balance;
    overflow-wrap: anywhere;
  }
  .member-text {
    color: var(--accent-member);
  }
  .funder-text {
    color: var(--accent-funder);
  }
  nav {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    margin-top: 40px;
  }
  @media (max-width: 480px) {
    .landing-page {
      padding: 48px 0;
    }
    h1 {
      font-size: clamp(30px, 8.8vw, 42px);
    }
    nav {
      flex-direction: column;
      align-items: stretch;
      margin-top: 32px;
      gap: 12px;
    }
  }
  @media (max-width: 480px) and (max-height: 650px) {
    .landing-page {
      padding: 16px 0;
    }
    nav {
      margin-top: 24px;
    }
  }
</style>
