<script lang="ts">
  import Icon from "./Icon.svelte";
  import { entityType, type Entity } from "./search";

  let { entity }: { entity: Entity } = $props();
  let hovered = $state(false);
  let focused = $state(false);
  let dismissed = $state(false);
</script>

<svelte:window
  onkeydown={(event) => {
    if (event.key === "Escape") dismissed = true;
  }}
/>

<span
  role="presentation"
  class="entity-indicator"
  onpointerenter={() => {
    hovered = true;
    dismissed = false;
  }}
  onpointerleave={() => (hovered = false)}
>
  <button
    type="button"
    class="entity-icon"
    aria-label={entityType(entity)}
    onfocus={() => {
      focused = true;
      dismissed = false;
    }}
    onblur={() => (focused = false)}
    onclick={(event) => {
      event.currentTarget.focus();
      dismissed = false;
    }}
  >
    <Icon name={entity.kind === "MP" ? "parliament" : "coin"} size={20} />
  </button>
  {#if (hovered || focused) && !dismissed}
    <span class="entity-tooltip" aria-hidden="true"
      ><span>{entityType(entity)}</span></span
    >
  {/if}
</span>
