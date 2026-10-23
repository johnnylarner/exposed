<script lang="ts">
  import type { IconName } from "./icon-types";
  let {
    name,
    size = 20,
  }: {
    name: IconName;
    size?: number;
  } = $props();

  type GlyphElement =
    | { tag: "path"; attributes: { d: string } }
    | { tag: "circle"; attributes: { cx: string; cy: string; r: string } };
  const glyphs = {
    search: [
      { tag: "circle", attributes: { cx: "10.75", cy: "10.75", r: "6.75" } },
      { tag: "path", attributes: { d: "m16 16 4.5 4.5" } },
    ],
    close: [{ tag: "path", attributes: { d: "m6 6 12 12M18 6 6 18" } }],
    parliament: [
      {
        tag: "path",
        attributes: { d: "m3 8 9-5 9 5H3ZM5 11v7m7-7v7m7-7v7M3 21h18" },
      },
    ],
    coin: [
      { tag: "circle", attributes: { cx: "12", cy: "12", r: "9" } },
      {
        tag: "path",
        attributes: { d: "M15 7h-2a3 3 0 0 0-3 3v4a3 3 0 0 1-2 3h8M8 12h6" },
      },
    ],
    person: [
      { tag: "circle", attributes: { cx: "12", cy: "8", r: "3.5" } },
      { tag: "path", attributes: { d: "M5 21v-2a7 7 0 0 1 14 0v2" } },
    ],
    building: [
      {
        tag: "path",
        attributes: {
          d: "M5 21V4a1 1 0 0 1 1-1h12a1 1 0 0 1 1 1v17M3 21h18M9 21v-4h6v4M9 7h1m4 0h1M9 11h1m4 0h1",
        },
      },
    ],
    group: [
      { tag: "circle", attributes: { cx: "9", cy: "8", r: "3" } },
      {
        tag: "path",
        attributes: {
          d: "M3 21v-3a6 6 0 0 1 12 0v3M16 5a3 3 0 0 1 0 6m2 3a5 5 0 0 1 3 4v3",
        },
      },
    ],
    flag: [
      {
        tag: "path",
        attributes: { d: "M5 21V3m0 1c5-3 9 3 14 0v10c-5 3-9-3-14 0" },
      },
    ],
    chart: [
      { tag: "path", attributes: { d: "M4 20h16M7 16v-5m5 5V5m5 11V8" } },
    ],
    question: [
      { tag: "circle", attributes: { cx: "12", cy: "12", r: "9" } },
      {
        tag: "path",
        attributes: {
          d: "M9.5 9a2.5 2.5 0 1 1 4.2 1.8c-.9.6-1.7 1.1-1.7 2.7M12 17h.01",
        },
      },
    ],
    retry: [
      { tag: "path", attributes: { d: "M20 11a8 8 0 1 0-2 6M20 4v7h-7" } },
    ],
    tag: [
      { tag: "path", attributes: { d: "M3 3h8l10 10-8 8L3 11V3Z" } },
      { tag: "circle", attributes: { cx: "7.5", cy: "7.5", r: "1" } },
    ],
  } satisfies Record<IconName, GlyphElement[]>;
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.6"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#each glyphs[name] as element}
    {#if element.tag === "path"}<path {...element.attributes} />{:else}<circle
        {...element.attributes}
      />{/if}
  {/each}
</svg>
