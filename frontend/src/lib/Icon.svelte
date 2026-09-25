<script lang="ts">
  let {
    name,
    size = 20,
  }: {
    name: "search" | "close" | "parliament" | "coin" | "retry";
    size?: number;
  } = $props();
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
  {#if name === "search"}
    <circle cx="10.75" cy="10.75" r="6.75" /><path d="m16 16 4.5 4.5" />
  {:else if name === "close"}
    <path d="m6 6 12 12M18 6 6 18" />
  {:else if name === "parliament"}
    <path d="m3 8 9-5 9 5H3ZM5 11v7m7-7v7m7-7v7M3 21h18" />
  {:else if name === "coin"}
    <circle cx="12" cy="12" r="9" />
    <path d="M15 7h-2a3 3 0 0 0-3 3v4a3 3 0 0 1-2 3h8M8 12h6" />
  {:else}
    <path d="M20 11a8 8 0 1 0-2 6M20 4v7h-7" />
  {/if}
</svg>
