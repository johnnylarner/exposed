<main id="landing" class="landing-page" tabindex="-1">
  <div class="landing-content">
    <h1>
      Explore UK MPs' declared financial interests and the people and
      organisations behind them.
    </h1>
    <nav aria-label="Explore Exposed">
      <a class="primary" href="/search">Search MPs and funders</a>
      <a href="/methodology">Methodology</a>
    </nav>
  </div>
</main>

<style>
  .landing-page {
    width: 100%;
    margin: 0;
    padding: 64px 0 80px;
    display: flex;
    align-items: center;
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
  nav {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    margin-top: 40px;
  }
  a {
    min-height: 48px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 12px 22px;
    border: 1px solid var(--line);
    border-radius: 6px;
    text-decoration: none;
    font-size: 14px;
    font-weight: 600;
  }
  a:hover {
    border-color: var(--accent);
  }
  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }
  .primary:hover {
    background: var(--accent-hover);
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
