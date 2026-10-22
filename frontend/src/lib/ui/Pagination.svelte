<script lang="ts">
  import Button from "./Button.svelte";
  let {
    page,
    pageCount,
    onprevious,
    onnext,
    label = "Declaration pages",
  }: {
    page: number;
    pageCount: number;
    onprevious: () => void;
    onnext: () => void;
    label?: string;
  } = $props();
</script>

<nav class="history-pagination" aria-label={label}>
  <Button variant="quiet" disabled={page <= 1} onclick={onprevious}
    >Previous</Button
  ><span>Page {page} of {pageCount}</span><Button
    variant="quiet"
    disabled={page >= pageCount}
    onclick={onnext}>Next</Button
  >
</nav>
