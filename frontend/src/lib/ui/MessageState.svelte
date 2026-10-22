<script lang="ts">
  import type { Snippet } from "svelte";
  import type { IconName } from "./icon-types";
  import Icon from "./Icon.svelte";
  let {
    title,
    level = 2,
    icon,
    variant = "centered",
    role,
    children,
    actions,
    class: className = "",
  }: {
    title: string;
    level?: 1 | 2;
    icon?: IconName;
    variant?: "centered" | "detail";
    role?: "alert" | "status";
    children?: Snippet;
    actions?: Snippet;
    class?: string;
  } = $props();
</script>

<div
  class={[
    "ui-message-state",
    variant === "centered" ? "message-state" : "detail-message",
    className,
  ]}
  {role}
>
  {#if icon}<span class="state-icon"><Icon name={icon} size={24} /></span
    >{/if}<svelte:element this={`h${level}`} class="message-title"
    >{title}</svelte:element
  >{#if children}{@render children()}{/if}{#if actions}{@render actions()}{/if}
</div>
