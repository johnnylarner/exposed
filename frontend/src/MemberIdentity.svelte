<script lang="ts">
  import { page } from "$app/state";
  import { summarizeMember, formatPounds } from "./lib/member-summary";
  import Icon from "./lib/Icon.svelte";
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

<header class="member-identity">
  <div class="member-identity-info">
    <div class="member-identity-title">
      <span class="profile-type"><Icon name="parliament" size={20} />MP</span>
      <h1>{member.name}</h1>
    </div>
    <p>
      {member.partyName || "Party unavailable"}<span
        >{member.membershipFrom || "Membership unavailable"}</span
      ><span>{member.isCurrentCommons ? "Current MP" : "Former MP"}</span>
    </p>
  </div>
  {#if view === "summary"}
    <div class="member-earnings-totals">
      <div class="earnings-values">
        <div class="member-value">
          <h2>Earnings this parliamentary term</h2>
          {#if summary.earningsTotals.currentTerm.declarationCount === 0}
            <p class="member-total unavailable">
              No declaration data available
            </p>
          {:else if summary.earningsTotals.currentTerm.amount === null}
            <p class="member-total unavailable">{formatPounds(null)}</p>
          {:else}
            <!-- svelte-ignore a11y_no_noninteractive_tabindex (Keyboard users need to scroll wide amounts without losing digits.) -->
            <div
              class="member-total"
              role="region"
              tabindex="0"
              aria-label="Earnings this parliamentary term in GBP"
              title={summary.earningsTotals.currentTerm.incomplete
                ? "Known GBP amounts only. Some amounts unavailable."
                : undefined}
            >
              {formatPounds(summary.earningsTotals.currentTerm.amount)}
            </div>
          {/if}
        </div>
        <div class="member-value">
          <h2>
            Historical earnings{summary.firstRegisteredAt
              ? ` since ${dateLabel(summary.firstRegisteredAt)}`
              : ""}
          </h2>
          {#if summary.earningsTotals.historical.declarationCount === 0}
            <p class="member-total unavailable">
              No declaration data available
            </p>
          {:else if summary.earningsTotals.historical.amount === null}
            <p class="member-total unavailable">{formatPounds(null)}</p>
          {:else}
            <!-- svelte-ignore a11y_no_noninteractive_tabindex (Keyboard users need to scroll wide amounts without losing digits.) -->
            <div
              class="member-total"
              role="region"
              tabindex="0"
              aria-label="Historical earnings in GBP"
              title={summary.earningsTotals.historical.incomplete
                ? "Known GBP amounts only. Some amounts unavailable."
                : undefined}
            >
              {formatPounds(summary.earningsTotals.historical.amount)}
            </div>
          {/if}
        </div>
      </div>
    </div>
  {/if}
  <nav class="member-tabs" aria-label="Member views">
    <a
      href={memberHref(member.id, "summary", page.url.searchParams)}
      aria-current={view === "summary" ? "page" : undefined}
      onclick={() =>
        posthog.capture("entity_view_selected", {
          entity_type: "member",
          selected_view: "summary",
        })}>Summary</a
    >
    <a
      href={memberHref(member.id, "history", page.url.searchParams)}
      aria-current={view === "history" ? "page" : undefined}
      onclick={() =>
        posthog.capture("entity_view_selected", {
          entity_type: "member",
          selected_view: "history",
        })}
      >Declaration history</a
    >
  </nav>
</header>
