<script lang="ts">
  import type { RankingRow } from "./ranking-types";
  let { rows, label }: { rows: RankingRow[]; label: string } = $props();
</script>

<ol class="ranking-list" role="list" aria-label={label}>
  {#each rows as row, index (row.key)}
    <li data-accent={row.accent} style:--row-delay={`${index * 35}ms`}>
      <span class="rank" aria-hidden="true">{index + 1}</span>
      <div class="identity">
        {#if row.href}<a href={row.href}>{row.name}</a>{:else}<span class="name"
            >{row.name}</span
          >{/if}
        <span class="context">{row.context}</span>
        <span class="bar-track" aria-hidden="true"
          ><span class:negative={row.negative} style:width={`${row.width}%`}
          ></span></span
        >
      </div>
      <div class="value">
        <span>{row.amount}</span>{#if row.incomplete}<span class="incomplete"
            >Incomplete</span
          >{/if}
      </div>
    </li>
  {/each}
</ol>

<style>
  .ranking-list {
    padding: 0;
    margin: 0;
    list-style: none;
  }
  li {
    display: grid;
    grid-template-columns: 24px minmax(0, 1fr) minmax(0, auto);
    align-items: start;
    gap: var(--space-3);
    padding: var(--space-2) 0;
    border-bottom: 1px solid var(--line);
    animation: ranking-enter 300ms both;
    animation-delay: var(--row-delay);
  }
  [data-accent="member"] {
    --row-accent: var(--accent-member);
  }
  [data-accent="funder"] {
    --row-accent: var(--accent-funder);
  }
  [data-accent="neutral"] {
    --row-accent: var(--muted);
  }
  .rank {
    color: var(--muted);
    font-size: var(--text-small);
    padding-top: 2px;
    font-variant-numeric: tabular-nums;
  }
  .identity {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    min-width: 0;
  }
  a,
  .name {
    color: var(--text);
    font-size: var(--text-body);
    font-weight: 600;
    overflow-wrap: anywhere;
    text-decoration: none;
  }
  a:hover {
    text-decoration: underline;
  }
  .context,
  .incomplete {
    color: var(--muted);
    font-size: var(--text-small);
  }
  .bar-track {
    display: block;
    height: 3px;
    margin-top: var(--space-1);
    background: var(--surface);
  }
  .bar-track > span {
    display: block;
    height: 100%;
    background: var(--row-accent);
    transform-origin: left;
    animation: ranking-bar 420ms both;
    animation-delay: var(--row-delay);
  }
  .bar-track > span.negative {
    opacity: 0.5;
  }
  .value {
    color: var(--row-accent);
    max-width: 330px;
    text-align: right;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--text-body);
    font-variant-numeric: tabular-nums;
    overflow-wrap: anywhere;
  }
  @keyframes ranking-enter {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
  @keyframes ranking-bar {
    from {
      transform: scaleX(0);
    }
    to {
      transform: scaleX(1);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    li,
    .bar-track > span {
      animation: none;
    }
  }
  @media (max-width: 480px) {
    li {
      grid-template-columns: 18px minmax(0, 1fr);
      gap: var(--space-2);
    }
    .value {
      grid-column: 2;
      text-align: left;
      max-width: 100%;
    }
  }
</style>
