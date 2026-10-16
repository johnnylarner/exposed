<script lang="ts">
  import App from "../../App.svelte";
  import type { PageData } from "./$types";
  let { data }: { data: PageData } = $props();
</script>

<App model={data.model} />
