<script lang="ts">
  import type { HTMLTextareaAttributes } from "svelte/elements";
  const generatedId = $props.id();
  let {
    label,
    hint,
    error,
    id = generatedId,
    defaultValue = "",
    value = $bindable(defaultValue),
    "aria-describedby": describedBy,
    class: className = "",
    ...attributes
  }: Omit<HTMLTextareaAttributes, "value" | "defaultValue" | "children"> & {
    label: string;
    hint?: string;
    error?: string;
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
  <textarea
    {...attributes}
    {id}
    {defaultValue}
    bind:value
    aria-describedby={description}
    aria-invalid={error ? true : attributes["aria-invalid"]}></textarea>
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
  textarea {
    width: 100%;
    min-width: 0;
    min-height: calc(var(--space-8) * 3);
    resize: vertical;
    padding: var(--space-3);
    border: 1px solid var(--line);
    border-radius: var(--radius-control);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    color-scheme: dark;
  }
  textarea:focus-visible {
    outline: var(--focus-width) solid var(--focus-color);
    outline-offset: var(--focus-offset);
  }
  textarea[aria-invalid="true"] {
    border-color: var(--warning);
  }
  textarea:disabled {
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
