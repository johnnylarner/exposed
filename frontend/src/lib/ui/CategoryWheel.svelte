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
    <svg class="wheel-sectors" viewBox="0 0 284 284" aria-hidden="true">
      {#each options as option}
        <path
          class="sector-surface"
          d={option.path}
          transform={`translate(${option.left} ${option.top})`}
          data-accent={option.accent}
        />
        <path
          class="sector-rim"
          class:selected={option.id === selected}
          d={option.rimPath}
          transform={`translate(${option.left} ${option.top})`}
          data-accent={option.accent}
        />
        {#if option.id === focused}<path
            class="focus-outline"
            d={option.path}
            transform={`translate(${option.left} ${option.top})`}
          />{/if}
      {/each}
    </svg>
    <span class="wheel-center" aria-hidden="true"
      ><Icon name="chart" size={30} /></span
    >
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
        style:width={`${option.width ?? 142}px`}
        style:height={`${option.height ?? 142}px`}
        style:clip-path={`path('${option.path}')`}
      >
        <span
          class="sector-label"
          style:left={`${option.x}px`}
          style:top={`${option.y}px`}
        >
          <Icon name={option.icon} size={32} />
          <span>{option.label}</span>
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
  .wheel-sectors {
    display: block;
    width: 284px;
    height: 284px;
    overflow: visible;
    pointer-events: none;
  }
  .sector-surface {
    fill: color-mix(in srgb, var(--sector-accent) 10%, var(--canvas));
    stroke: var(--line);
    stroke-width: 1;
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
  .sector-rim {
    fill: none;
    stroke: color-mix(in srgb, var(--sector-accent) 25%, var(--line));
    stroke-width: 2;
    stroke-linecap: round;
    transition: stroke 180ms;
  }
  .sector-rim.selected {
    stroke: var(--sector-accent);
    stroke-width: 3;
  }
  .wheel-center {
    position: absolute;
    left: 142px;
    top: 142px;
    transform: translate(-50%, -50%);
    color: var(--muted);
    pointer-events: none;
    display: flex;
  }
  path.focus-outline {
    fill: none;
    stroke: var(--focus-color);
    stroke-width: 6;
  }
  button {
    position: absolute;
    background: transparent;
    border: 0;
    padding: 0;
    color: color-mix(in srgb, var(--sector-accent) 80%, var(--muted));
    font: inherit;
    font-size: 17px;
    font-weight: 600;
    cursor: pointer;
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
    flex: none;
  }
  @media (prefers-reduced-motion: reduce) {
    path {
      transition: none;
    }
  }
</style>
