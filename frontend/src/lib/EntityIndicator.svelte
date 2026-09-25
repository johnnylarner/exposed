<script lang="ts">
  import Icon from "./Icon.svelte";
  import {
    entityType,
    funderKindLabel,
    funderKindIcon,
    type Entity,
  } from "./search";

  let {
    entity,
    detail = "role",
  }: { entity: Entity; detail?: "role" | "kind" } = $props();
  const label = $derived(
    detail === "kind" ? funderKindLabel(entity.funderKind) : entityType(entity),
  );
  const icon = $derived(
    detail === "kind"
      ? funderKindIcon(entity.funderKind)
      : entity.kind === "MP"
        ? "parliament"
        : "coin",
  );
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
  class:entity-kind={detail === "kind"}
  onpointerenter={() => {
    hovered = true;
    dismissed = false;
  }}
  onpointerleave={() => (hovered = false)}
>
  <button
    type="button"
    class="entity-icon"
    aria-label={label}
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
    <Icon name={icon} size={detail === "kind" ? 18 : 20} />
  </button>
  {#if (hovered || focused) && !dismissed}
    <span class="entity-tooltip" aria-hidden="true"><span>{label}</span></span>
  {/if}
</span>
