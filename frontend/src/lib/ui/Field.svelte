<script lang="ts">
  import type { HTMLInputAttributes } from "svelte/elements";
  const generatedId = $props.id();
  let {
    label,
    hint,
    error,
    id = generatedId,
    type = "text",
    defaultValue = "",
    value = $bindable(defaultValue),
    "aria-describedby": describedBy,
    class: className = "",
    ...attributes
  }: Omit<
    HTMLInputAttributes,
    "type" | "value" | "defaultValue" | "children"
  > & {
    label: string;
    hint?: string;
    error?: string;
    type?: "text" | "search" | "date";
    value?: string;
    defaultValue?: string;
  } = $props();
  const description = $derived(
    [
      describedBy,
      hint ? `${id}-hint` : undefined,
      error ? `${id}-error` : undefined,
    ]
      .filter(Boolean)
      .join(" ") || undefined,
  );
</script>

<div class={["ui-field", className]}>
  <label for={id}>{label}</label>
  <input
    {...attributes}
    {id}
    {type}
    {defaultValue}
    bind:value
    aria-describedby={description}
    aria-invalid={error ? true : attributes["aria-invalid"]}
  />
  {#if hint}<p id={`${id}-hint`} class="hint">{hint}</p>{/if}
  {#if error}<p id={`${id}-error`} class="error">{error}</p>{/if}
</div>

<style>
  .ui-field {
    display: grid;
    gap: var(--space-2);
    min-width: 0;
    font-size: var(--text-body);
  }
  label {
    font-weight: 500;
  }
  input {
    width: 100%;
    min-width: 0;
    min-height: 44px;
    padding: 10px 12px;
    border: 1px solid var(--line);
    border-radius: var(--radius-control);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    color-scheme: dark;
  }
  input:focus-visible {
    outline: var(--focus-width) solid var(--focus-color);
    outline-offset: var(--focus-offset);
  }
  input[aria-invalid="true"] {
    border-color: var(--warning);
  }
  input:disabled {
    opacity: 0.4;
  }
  p {
    margin: 0;
    font-size: var(--text-small);
    line-height: 1.6;
  }
  .hint {
    color: var(--muted);
  }
  .error {
    color: var(--warning);
  }
</style>
