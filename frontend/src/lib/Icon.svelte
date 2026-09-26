<script lang="ts">
  let {
    name,
    size = 20,
  }: {
    name:
      | "search"
      | "close"
      | "parliament"
      | "coin"
      | "retry"
      | "person"
      | "building"
      | "group"
      | "question"
      | "tag";
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
  {:else if name === "person"}
    <circle cx="12" cy="8" r="3.5" /><path d="M5 21v-2a7 7 0 0 1 14 0v2" />
  {:else if name === "building"}
    <path
      d="M5 21V4a1 1 0 0 1 1-1h12a1 1 0 0 1 1 1v17M3 21h18M9 21v-4h6v4M9 7h1m4 0h1M9 11h1m4 0h1"
    />
  {:else if name === "group"}
    <circle cx="9" cy="8" r="3" /><path
      d="M3 21v-3a6 6 0 0 1 12 0v3M16 5a3 3 0 0 1 0 6m2 3a5 5 0 0 1 3 4v3"
    />
  {:else if name === "question"}
    <circle cx="12" cy="12" r="9" /><path
      d="M9.5 9a2.5 2.5 0 1 1 4.2 1.8c-.9.6-1.7 1.1-1.7 2.7M12 17h.01"
    />
  {:else if name === "tag"}
    <path d="M3 3h8l10 10-8 8L3 11V3Z" /><circle cx="7.5" cy="7.5" r="1" />
  {:else}
    <path d="M20 11a8 8 0 1 0-2 6M20 4v7h-7" />
  {/if}
</svg>
