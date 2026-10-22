<script lang="ts">
  import { onDestroy } from "svelte";
  import type { PieSlice } from "./chart-types";
  let { slices, label }: { slices: PieSlice[]; label: string } = $props();
  const instanceId = $props.id();
  let tooltip = $state<{ index: number; left: number; top: number } | null>(
    null,
  );
  let hoverDismissed = false;
  let anchor: Element | null = null;
  let anchorTop = 0;
  let dismissal: ReturnType<typeof setTimeout> | undefined;
  function keepOpen() {
    clearTimeout(dismissal);
  }
  function show(index: number, target: EventTarget | null) {
    if (!(target instanceof Element)) return;
    keepOpen();
    anchor = target.closest("svg") ?? target;
    const bounds = anchor.getBoundingClientRect();
    anchorTop = bounds.top;
    tooltip = {
      index,
      left: Math.max(12, Math.min(bounds.left, window.innerWidth - 292)),
      top:
        bounds.bottom + 100 > window.innerHeight
          ? Math.max(12, bounds.top - 100)
          : bounds.bottom + 8,
    };
  }
  function hide() {
    keepOpen();
    tooltip = null;
  }
  function dismissIfAnchorMoved() {
    if (anchor?.getBoundingClientRect().top !== anchorTop) hide();
  }
  onDestroy(keepOpen);
  function dismiss() {
    dismissal = setTimeout(() => {
      tooltip = null;
    }, 120);
  }
  function escape(event: KeyboardEvent) {
    if (event.key === "Escape" && tooltip !== null) {
      hoverDismissed = true;
      keepOpen();
      tooltip = null;
    }
  }
</script>

<svelte:window
  onkeydown={escape}
  onscroll={dismissIfAnchorMoved}
  onresize={hide}
/>
<div class="funder-pie">
  <svg viewBox="0 0 200 200" role="img" aria-label={label}>
    {#if slices.every((slice) => slice.share === 0)}<circle
        cx="100"
        cy="100"
        r="90"
        fill="#ece8e1"
      />{/if}
    {#each slices as slice, index}
      {#if slice.share > 0}
        <g
          role="img"
          aria-label={`${slice.label}: ${slice.value}`}
          aria-describedby={tooltip?.index === index
            ? `${instanceId}-tooltip`
            : undefined}
          onpointermove={(event) => {
            if (
              hoverDismissed &&
              (event.movementX || event.movementY) &&
              event.pointerType !== "touch"
            ) {
              hoverDismissed = false;
              show(index, event.currentTarget);
            }
          }}
          onpointerenter={(event) => {
            if (event.pointerType !== "touch" && !hoverDismissed)
              show(index, event.currentTarget);
          }}
          onpointerleave={dismiss}
        >
          {#if slice.share === 1}<circle
              cx="100"
              cy="100"
              r="90"
              fill={slice.color}
            />
          {:else}<path d={slice.path} fill={slice.color} />{/if}
        </g>
      {/if}
    {/each}
  </svg>
</div>
<ul class="funder-party-legend" onscroll={dismissIfAnchorMoved}>
  {#each slices as slice, index}<li
      aria-label={`${slice.label}: ${slice.value}`}
      aria-describedby={tooltip?.index === index
        ? `${instanceId}-tooltip`
        : undefined}
      onpointermove={(event) => {
        if (
          hoverDismissed &&
          (event.movementX || event.movementY) &&
          event.pointerType !== "touch"
        ) {
          hoverDismissed = false;
          show(index, event.currentTarget);
        }
      }}
      onpointerenter={(event) => {
        if (event.pointerType !== "touch" && !hoverDismissed)
          show(index, event.currentTarget);
      }}
      onpointerleave={dismiss}
    >
      <span class="party-swatch" style:background={slice.color}></span>
      <div>
        <span class="party-name">{slice.label}</span><span class="entry-count"
          >{slice.detail}</span
        >
      </div>
    </li>{/each}
</ul>
{#if tooltip}{@const slice = slices[tooltip.index]}
  <div
    id={`${instanceId}-tooltip`}
    class="party-tooltip"
    role="tooltip"
    style:left={`${tooltip.left}px`}
    style:top={`${tooltip.top}px`}
    onpointerenter={keepOpen}
    onpointerleave={dismiss}
  >
    <span>{slice.label}</span>
    <strong>{slice.value}</strong>
    <span class="entry-count">{slice.detail}</span>
  </div>
{/if}
