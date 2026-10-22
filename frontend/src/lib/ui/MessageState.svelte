<script lang="ts">
  import type { Snippet } from "svelte";
  import type { IconName } from "./icon-types";
  import Icon from "./Icon.svelte";
  let {
    title,
    level = 2,
    icon,
    iconClass = "state-icon",
    role,
    children,
    actions,
    class: className = "message-state",
  }: {
    title: string;
    level?: 1 | 2;
    icon?: IconName;
    iconClass?: string;
    role?: "alert" | "status";
    children?: Snippet;
    actions?: Snippet;
    class?: string;
  } = $props();
</script>

<div class={className} {role}>
  {#if icon}<span class={iconClass}><Icon name={icon} size={24} /></span
    >{/if}<svelte:element this={`h${level}`}>{title}</svelte:element
  >{#if children}{@render children()}{/if}{#if actions}{@render actions()}{/if}
</div>
