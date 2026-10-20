<script lang="ts">
  import { page } from "$app/state";
  import {
    CURRENT_PARLIAMENT_START,
    summarizeMember,
    formatPounds,
  } from "./lib/member-summary";
  import Icon from "./lib/Icon.svelte";
  import { memberHref, type MemberProfile } from "./lib/entity-details";
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
          <span>Earnings this parliamentary term</span>
          <p class="member-total">
            {formatPounds(summary.earningsTotals.currentTerm.amount)}
          </p>
          <p class="partial-note">
            Since {dateLabel(CURRENT_PARLIAMENT_START)}
          </p>
          {#if summary.earningsTotals.currentTerm.incomplete}<p
              class="partial-note"
            >
              Some amounts unavailable
            </p>{/if}
        </div>
        <div class="member-value">
          <span>Historical earnings</span>
          <p class="member-total">
            {formatPounds(summary.earningsTotals.historical.amount)}
          </p>
          <p class="partial-note">All stored earnings, including this term</p>
          {#if summary.earningsTotals.historical.incomplete}<p
              class="partial-note"
            >
              Some amounts unavailable
            </p>{/if}
        </div>
      </div>
      <p class="partial-note">
        Known GBP amounts, based on registration date. Other currencies
        excluded.
      </p>
      <p class="partial-note">
        {summary.firstRegisteredAt
          ? `Earliest stored declaration across all categories: ${dateLabel(summary.firstRegisteredAt)}.`
          : "No dated declarations stored."}
      </p>
      {#if summary.undated}<p class="partial-note">
          Earnings without a registration date are included in historical
          earnings and excluded from this term.
        </p>{/if}
    </div>
  {/if}
  <nav class="member-tabs" aria-label="Member views">
    <a
      href={memberHref(member.id, "summary", page.url.searchParams)}
      aria-current={view === "summary" ? "page" : undefined}>Summary</a
    >
    <a
      href={memberHref(member.id, "history", page.url.searchParams)}
      aria-current={view === "history" ? "page" : undefined}
      >Declaration history</a
    >
  </nav>
</header>
