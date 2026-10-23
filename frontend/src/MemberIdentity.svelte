<script lang="ts">
  import {
    Icon,
    Metric,
    IdentityHeader,
    ViewLinks,
    ViewLink,
    Badge,
  } from "./lib/ui";
  import { page } from "$app/state";
  import { summarizeMember, formatPounds } from "./lib/member-summary";
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
  function dateLabel(value: string) {
    return new Date(value).toLocaleDateString("en-GB", {
      day: "numeric",
      month: "long",
      year: "numeric",
      timeZone: "UTC",
    });
  }
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
      <div class="member-earnings-totals">
        <div class="earnings-values">
          {#each [{ label: "Declared amounts this parliamentary term", regionLabel: "Declared amounts this parliamentary term in GBP", total: summary.declaredTotals.currentTerm }, { regionLabel: "Historical declared amounts in GBP", label: `Historical declared amounts${summary.firstRegisteredAt ? ` since ${dateLabel(summary.firstRegisteredAt)}` : ""}`, total: summary.declaredTotals.historical }, { label: `Declared amounts in ${summary.declaredTotals.calendarYear}`, regionLabel: `Declared amounts in ${summary.declaredTotals.calendarYear} in GBP`, total: summary.declaredTotals.currentYear }] as metric}<Metric
              variant="expanded"
              regionLabel={metric.regionLabel}
              label={metric.label}
              value={metric.total.declarationCount === 0
                ? "No declarations recorded"
                : formatPounds(metric.total.amount)}
              available={metric.total.declarationCount > 0 &&
                metric.total.amount !== null}
              description={metric.total.incomplete
                ? "Known GBP amounts only. Some amounts unavailable."
                : undefined}
            />{/each}
        </div>
      </div>
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
