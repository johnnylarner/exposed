<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";
  import type {
    RegisteredInterestStatus,
    RegisteredInterestVariant,
  } from "./registered-interest-types";
  let {
    text,
    status,
    correction = false,
    variant = "default",
    metadata,
    children,
  }: {
    text: string;
    status: RegisteredInterestStatus;
    correction?: boolean;
    variant?: RegisteredInterestVariant;
    metadata?: Snippet;
    children?: Snippet;
  } = $props();
</script>

{#snippet content()}
  <p class="interest-text">{text}</p>
  {#if status.kind === "deleted"}
    <p class="interest-status">
      Deleted <time datetime={status.datetime}>{status.date}</time>
    </p>
  {/if}
  {#if correction}<p class="interest-correction">Correction</p>{/if}
  {#if metadata}<p class="interest-metadata">{@render metadata()}</p>{/if}
{/snippet}

<li class="registered-interest">
  <div
    class="interest-content"
    class:company={variant === "company"}
    data-deleted={status.kind === "deleted"}
  >
    {#if variant === "company"}
      <span class="company-icon"><Icon name="building" /></span>
      <div class="company-text">
        {@render content()}
      </div>
    {:else}
      {@render content()}
    {/if}
  </div>
  {#if children}{@render children()}{/if}
</li>

<style>
  .registered-interest {
    min-width: 0;
  }
  .interest-content {
    overflow-wrap: anywhere;
    line-height: 1.6;
  }
  .company {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: var(--space-3);
    padding: var(--space-4) var(--space-3);
    border-bottom: 1px solid var(--line);
  }
  .company-icon {
    color: var(--muted);
    display: flex;
    align-items: center;
    height: calc(var(--text-item) * 1.6);
  }
  .company-text {
    min-width: 0;
  }
  .company .interest-text {
    font-size: var(--text-item);
    font-weight: 600;
  }
  p {
    margin: 0;
  }
  .interest-text {
    white-space: pre-wrap;
  }
  .interest-content[data-deleted="true"] {
    color: var(--muted);
  }
  .interest-status,
  .interest-correction,
  .interest-metadata {
    color: var(--muted);
    font-size: var(--text-small);
    line-height: 1.6;
    margin-top: var(--space-1);
  }
</style>
