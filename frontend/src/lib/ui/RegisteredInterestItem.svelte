<script lang="ts">
  import type { Snippet } from "svelte";
  import Disclosure from "./Disclosure.svelte";
  import type {
    RegisteredInterestPresentation,
    RegisteredInterestStatus,
  } from "./registered-interest-types";
  let {
    text,
    presentation,
    status,
    correction = false,
    metadata,
    children,
  }: {
    text: string;
    presentation: RegisteredInterestPresentation;
    status: RegisteredInterestStatus;
    correction?: boolean;
    metadata?: Snippet;
    children?: Snippet;
  } = $props();
</script>

<li class="registered-interest" data-presentation={presentation.kind}>
  <div class="interest-content" data-deleted={status.kind === "deleted"}>
    {#if presentation.kind === "compact"}
      <Disclosure variant="plain" summary={presentation.summary}>
        <div class="interest-source">
          <p class="interest-text">{text}</p>
          {#if metadata}<p class="interest-metadata">
              {@render metadata()}
            </p>{/if}
        </div>
      </Disclosure>
    {:else}
      <p class="interest-text">{text}</p>
    {/if}
    {#if status.kind === "deleted"}
      <p class="interest-status">
        Deleted <time datetime={status.datetime}>{status.date}</time>
      </p>
    {/if}
    {#if correction}<p class="interest-correction">Correction</p>{/if}
    {#if presentation.kind === "full" && metadata}
      <div class="interest-details">
        <Disclosure variant="plain" summary="Details">
          <div class="interest-source">
            <p class="interest-metadata">{@render metadata()}</p>
          </div>
        </Disclosure>
      </div>
    {/if}
  </div>
  {#if children}{@render children()}{/if}
</li>

<style>
  .registered-interest {
    min-width: 0;
  }
  .registered-interest[data-presentation="compact"] {
    list-style: none;
  }
  .interest-content {
    overflow-wrap: anywhere;
    line-height: 1.6;
  }
  p {
    margin: 0;
  }
  .interest-text {
    white-space: pre-wrap;
  }
  .interest-source {
    margin-top: var(--space-3);
  }
  .interest-content[data-deleted="true"] {
    color: var(--muted);
  }
  .interest-status,
  .interest-correction,
  .interest-details,
  .interest-metadata {
    color: var(--muted);
    font-size: var(--text-small);
    line-height: 1.6;
    margin-top: var(--space-1);
  }
</style>
