<script lang="ts">
  import { onMount } from "svelte";
  import type { Snippet } from "svelte";

  let { changeKey, children }: { changeKey: string; children: Snippet } =
    $props();
  let root: HTMLDivElement;
  let preference = $state<MediaQueryList>();
  let previousKey: string;
  let animation: Animation | undefined;

  onMount(() => {
    previousKey = changeKey;
    const query = matchMedia("(prefers-reduced-motion: reduce)");
    preference = query;
    function cancelReducedMotion() {
      if (query.matches) animation?.cancel();
    }
    query.addEventListener("change", cancelReducedMotion);
    return () => {
      query.removeEventListener("change", cancelReducedMotion);
      animation?.cancel();
    };
  });

  $effect(() => {
    const key = changeKey;
    if (!preference || key === previousKey) return;
    previousKey = key;
    animation?.cancel();
    if (!preference.matches) {
      animation = root.animate([{ opacity: 0.65 }, { opacity: 1 }], {
        duration: 200,
        easing: "ease-out",
      });
    }
  });
</script>

<div bind:this={root} class="content-change">
  {@render children()}
</div>
