<script lang="ts">
  import Icon from "./Icon.svelte";
  import type { IconName } from "./icon-types";
  let { label, icon }: { label: string; icon: IconName } = $props();
  const id = $props.id();
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
    aria-label={label}
    aria-describedby={(hovered || focused) && !dismissed
      ? `${id}-tooltip`
      : undefined}
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
    <Icon name={icon} size={20} />
  </button>
  {#if (hovered || focused) && !dismissed}
    <span class="entity-tooltip" id={`${id}-tooltip`} aria-hidden="true"
      ><span>{label}</span></span
    >
  {/if}
</span>
