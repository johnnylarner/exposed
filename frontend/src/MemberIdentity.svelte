<script lang="ts">
  import DeclaredTotals from "./DeclaredTotals.svelte";
  import { Icon, IdentityHeader, ViewLinks, ViewLink, Badge } from "./lib/ui";
  import { page } from "$app/state";
  import { summarizeMember } from "./lib/member-summary";
  import { memberHref, type MemberProfile } from "./lib/entity-details";
  import posthog from "posthog-js";
  let {
    member,
    view,
    summary,
  }: {
    member: MemberProfile;
    view: "summary" | "history";
    summary: ReturnType<typeof summarizeMember>;
  } = $props();
</script>

<IdentityHeader name={member.name}
  >{#snippet kind()}<Badge variant="profile"
      ><Icon name="parliament" size={20} />MP</Badge
    >{/snippet}{#snippet metadata()}
    {member.partyName || "Party unavailable"}<span
      >{member.membershipFrom || "Membership unavailable"}</span
    ><span>{member.isCurrentCommons ? "Current MP" : "Former MP"}</span>
  {/snippet}{#snippet values()}
    {#if view === "summary"}
      <DeclaredTotals
        totals={summary.declaredTotals}
        firstRegisteredAt={summary.firstRegisteredAt}
      />
    {/if}
  {/snippet}{#snippet navigation()}<ViewLinks label="Member views">
      <ViewLink
        href={memberHref(member.id, "summary", page.url.searchParams)}
        aria-current={view === "summary" ? "page" : undefined}
        onclick={() =>
          posthog.capture("entity_view_selected", {
            entity_type: "member",
            selected_view: "summary",
          })}>Summary</ViewLink
      >
      <ViewLink
        href={memberHref(member.id, "history", page.url.searchParams)}
        aria-current={view === "history" ? "page" : undefined}
        onclick={() =>
          posthog.capture("entity_view_selected", {
            entity_type: "member",
            selected_view: "history",
          })}>Declaration history</ViewLink
      >
    </ViewLinks>{/snippet}</IdentityHeader
>
