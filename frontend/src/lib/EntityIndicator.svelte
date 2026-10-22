<script lang="ts">
  import { Icon, Badge, IconIndicator } from "./ui";
  import {
    entityType,
    funderKindLabel,
    funderKindIcon,
    type Entity,
  } from "./search";

  let {
    entity,
    detail = "role",
  }: { entity: Entity; detail?: "role" | "kind" } = $props();
  const label = $derived(
    detail === "kind" ? funderKindLabel(entity.funderKind) : entityType(entity),
  );
  const icon = $derived(
    detail === "kind"
      ? funderKindIcon(entity.funderKind)
      : entity.kind === "MP"
        ? "parliament"
        : "coin",
  );
</script>

{#if detail === "kind"}<Badge
    ><Icon name={icon} size={16} /><span class="entity-kind-label">{label}</span
    ></Badge
  >{:else}<IconIndicator {label} {icon} />{/if}
