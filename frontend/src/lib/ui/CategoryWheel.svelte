<script lang="ts" generics="Id extends string">
  import Icon from "./Icon.svelte";
  import type { CategoryWheelOption } from "./ranking-types";
  let {
    options,
    selected,
    onselect,
    label,
    size = 284,
  }: {
    options: CategoryWheelOption<Id>[];
    selected: Id;
    onselect: (id: Id) => void;
    label: string;
    size?: number;
  } = $props();
  let focused = $state<Id | null>(null);
</script>

<div
  class="category-wheel"
  role="group"
  aria-label={label}
  style:width={`${size}px`}
  style:height={`${size}px`}
>
  <div class="wheel-canvas" style:transform={`scale(${size / 284})`}>
    <svg viewBox="0 0 284 284" aria-hidden="true">
      {#each options as option}
        {#if option.id === focused}<path
            class="focus-outline"
            d={option.path}
            transform={`translate(${option.left} ${option.top})`}
          />{/if}
        <path
          d={option.path}
          transform={`translate(${option.left} ${option.top})`}
          class:selected={option.id === selected}
          data-accent={option.accent}
        />
      {/each}
    </svg>
    {#each options as option}
      <button
        type="button"
        aria-pressed={selected === option.id}
        onclick={() => onselect(option.id)}
        onfocus={(event) =>
          (focused = event.currentTarget.matches(":focus-visible")
            ? option.id
            : null)}
        onblur={() => (focused = null)}
        data-accent={option.accent}
        style:left={`${option.left}px`}
        style:top={`${option.top}px`}
        style:clip-path={`path('${option.path}')`}
      >
        <span
          class="sector-label"
          style:left={`${option.x}px`}
          style:top={`${option.y}px`}
        >
          <Icon name={option.icon} size={20} />
          <span>{option.label}</span>
          <span class="selection-mark" aria-hidden="true"
            >{selected === option.id ? "✓" : ""}</span
          >
        </span>
      </button>
    {/each}
  </div>
</div>

<style>
  .category-wheel {
    position: relative;
    aspect-ratio: 1;
    flex: none;
  }
  .wheel-canvas {
    position: relative;
    width: 284px;
    height: 284px;
    transform-origin: top left;
  }
  svg {
    display: block;
    width: 284px;
    height: 284px;
    overflow: visible;
    pointer-events: none;
  }
  path {
    fill: var(--surface);
    stroke: var(--line);
    stroke-width: 1.5;
    transition:
      fill 180ms,
      stroke 180ms;
  }
  [data-accent="member"] {
    --sector-accent: var(--accent-member);
  }
  [data-accent="funder"] {
    --sector-accent: var(--accent-funder);
  }
  [data-accent="neutral"] {
    --sector-accent: var(--muted);
  }
  path.selected {
    stroke: var(--sector-accent);
    stroke-width: 2;
    fill: color-mix(in srgb, var(--sector-accent) 13%, var(--canvas));
  }
  path.focus-outline {
    fill: none;
    stroke: var(--focus-color);
    stroke-width: 6;
  }
  button {
    position: absolute;
    width: 142px;
    height: 142px;
    background: transparent;
    border: 0;
    padding: 0;
    color: var(--text);
    font: inherit;
    font-size: 15px;
    cursor: pointer;
  }
  button[aria-pressed="true"] {
    color: var(--sector-accent);
  }
  .sector-label {
    position: absolute;
    transform: translate(-50%, -50%);
    width: 110px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-1);
  }
  button :global(svg) {
    color: var(--muted);
    flex: none;
  }
  .selection-mark {
    height: 14px;
    line-height: 14px;
    font-weight: 700;
  }
  @media (prefers-reduced-motion: reduce) {
    path {
      transition: none;
    }
  }
</style>
