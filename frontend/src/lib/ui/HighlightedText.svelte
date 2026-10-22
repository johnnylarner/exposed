<script lang="ts">
  let { parts }: { parts: { text: string; matched: boolean }[] } = $props();
</script>

{#each parts as part}{#if part.matched}<mark>{part.text}</mark
    >{:else}{part.text}{/if}{/each}
