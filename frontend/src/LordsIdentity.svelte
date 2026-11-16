<script lang="ts">
  import { page } from "$app/state";
  import { lordHref, type DetailRoute } from "./lib/entity-details";
  import { sourceCalendarDate, type LordsDetails } from "./lib/lords";
  import { Badge, Icon, IdentityHeader, ViewLinks, ViewLink } from "./lib/ui";
  import posthog from "posthog-js";
  let {
    member,
    view,
  }: {
    member: LordsDetails["member"];
    view: Extract<DetailRoute, { kind: "lord" }>["view"];
  } = $props();
  const memberMetadata = $derived(
    [member.partyName, member.membershipFrom, member.membershipStatus].filter(
      (value) => value !== null && value !== "",
    ),
  );
</script>

<IdentityHeader name={member.name}>
  {#snippet kind()}<Badge variant="profile"
      ><Icon name="portcullis" size={20} />Lords</Badge
    >{/snippet}
  {#snippet metadata()}
    {#each memberMetadata as value, index}
      {#if index === 0}{value}{:else}<span>{value}</span>{/if}
    {/each}
    {#if member.membershipEndDate}
      {#snippet ended(date: string)}Membership ended <time datetime={date}
          >{sourceCalendarDate(date)}</time
        >{/snippet}
      {#if memberMetadata.length}<span
          >{@render ended(member.membershipEndDate)}</span
        >{:else}{@render ended(member.membershipEndDate)}{/if}
    {/if}
  {/snippet}
  {#snippet navigation()}
    <ViewLinks label="Lords views">
      <ViewLink
        href={lordHref(member.id, "summary", page.url.searchParams)}
        aria-current={view === "summary" ? "page" : undefined}
        onclick={() =>
          posthog.capture("entity_view_selected", {
            entity_type: "lord",
            selected_view: "summary",
          })}>Summary</ViewLink
      >
      <ViewLink
        href={lordHref(member.id, "history", page.url.searchParams)}
        aria-current={view === "history" ? "page" : undefined}
        onclick={() =>
          posthog.capture("entity_view_selected", {
            entity_type: "lord",
            selected_view: "history",
          })}>Interest history</ViewLink
      >
    </ViewLinks>
  {/snippet}
</IdentityHeader>
