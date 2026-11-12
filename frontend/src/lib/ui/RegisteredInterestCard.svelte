<script lang="ts">
  import type { Snippet } from "svelte";
  import type { RegisteredInterestStatus } from "./registered-interest-types";
  let {
    text,
    status,
    correction = false,
    metadata,
    children,
  }: {
    text: string;
    status: RegisteredInterestStatus;
    correction?: boolean;
    metadata?: Snippet;
    children?: Snippet;
  } = $props();
</script>

<li class="registered-interest">
  <div class="interest-content" data-deleted={status.kind === "deleted"}>
    {#if status.kind === "deleted"}
      <p class="interest-status">
        Deleted <time datetime={status.datetime}>{status.date}</time>
      </p>
    {/if}
    <p class="interest-text">{text}</p>
    {#if correction}<p class="interest-correction">Correction</p>{/if}
    {#if metadata}<p class="interest-metadata">{@render metadata()}</p>{/if}
  </div>
  {#if children}{@render children()}{/if}
</li>

<style>
  .registered-interest {
    min-width: 0;
  }
  .interest-content {
    padding: var(--space-4);
    border: 1px solid var(--line);
    border-radius: var(--radius-control);
    background: var(--surface);
  }
  .interest-content[data-deleted="true"] {
    border-style: dashed;
  }
  p {
    margin: 0;
  }
  .interest-text {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    line-height: 1.8;
  }
  .interest-content[data-deleted="true"] .interest-text {
    color: var(--muted);
  }
  .interest-status {
    color: var(--muted);
    font-size: var(--text-small);
    margin-bottom: var(--space-2);
    font-weight: 650;
  }
  .interest-correction,
  .interest-metadata {
    color: var(--muted);
    font-size: var(--text-small);
    line-height: 1.6;
    margin-top: var(--space-2);
  }
</style>
