<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLInputAttributes } from "svelte/elements";
  import Icon from "./Icon.svelte";
  const generatedId = $props.id();
  let {
    input = $bindable(),
    defaultValue = "",
    value = $bindable(defaultValue),
    label,
    help,
    shortcut,
    onclear,
    "aria-describedby": describedBy,
    disabled,
    readonly,
    id = generatedId,
    ...attributes
  }: Omit<
    HTMLInputAttributes,
    "children" | "value" | "defaultValue" | "type"
  > & {
    input?: HTMLInputElement;
    value?: string;
    defaultValue?: string;
    label: string;
    help: Snippet;
    shortcut?: string;
    onclear: () => void;
  } = $props();
</script>

<label class="visually-hidden" for={id}>{label}</label>
<div class="search-field">
  <span class="search-icon"><Icon name="search" size={24} /></span><input
    {...attributes}
    {id}
    type="search"
    {disabled}
    {readonly}
    {defaultValue}
    bind:this={input}
    bind:value
    aria-describedby={[describedBy, `${id}-help`].filter(Boolean).join(" ")}
  />{#if value}<button
      class="clear-button"
      disabled={disabled || readonly}
      type="button"
      aria-label="Clear search"
      onclick={onclear}><Icon name="close" size={18} /></button
    >{:else if shortcut}<kbd class="search-shortcut" aria-hidden="true"
      >{shortcut}</kbd
    >{/if}
</div>
<p class="search-help" id={`${id}-help`}>{@render help()}</p>
