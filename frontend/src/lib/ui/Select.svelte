<script lang="ts" generics="Value extends string">
  import type { HTMLSelectAttributes } from "svelte/elements";
  let {
    label,
    options,
    value,
    onselect,
    ...attributes
  }: Omit<
    HTMLSelectAttributes,
    "value" | "onchange" | "children" | "multiple" | "onselect"
  > & {
    label: string;
    options: readonly { value: Value; label: string }[];
    value: Value;
    onselect: (value: Value) => void;
  } = $props();
</script>

<select
  {...attributes}
  aria-label={label}
  {value}
  onchange={(event) => {
    const option = options.find(
      (option) => option.value === event.currentTarget.value,
    );
    event.currentTarget.value = value;
    if (option) onselect(option.value);
  }}
>
  {#each options as option}<option value={option.value}>{option.label}</option
    >{/each}
</select>

<style>
  select {
    font: inherit;
    font-size: var(--text-body);
    color: var(--action);
    background: var(--action-surface);
    border: 1px solid var(--action-border);
    border-radius: var(--radius-control);
    padding: var(--space-2) var(--space-3);
    max-width: 100%;
  }
  select:focus-visible {
    outline: var(--focus-width) solid var(--focus-color);
    outline-offset: var(--focus-offset);
  }
</style>
