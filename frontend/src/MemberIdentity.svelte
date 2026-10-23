<script lang="ts">
  import {
    Icon,
    DataTable,
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
      <div class="member-declared-totals">
        <DataTable caption="Declared GBP amounts">
          {#snippet head()}<tr
              ><th scope="col">Period</th><th scope="col" class="declared-total"
                >Declared amount (GBP)</th
              ></tr
            >{/snippet}
          {#each [{ label: `Total declared${summary.firstRegisteredAt ? ` (${dateLabel(summary.firstRegisteredAt)})` : ""}`, total: summary.declaredTotals.historical }, { label: "Declared this term", total: summary.declaredTotals.currentTerm }, { label: `Declared in ${summary.declaredTotals.calendarYear}`, total: summary.declaredTotals.currentYear }] as row}
            <tr>
              <th scope="row">{row.label}</th>
              <td class="declared-total">
                {#if row.total.declarationCount > 0 && row.total.amount !== null}
                  <!-- svelte-ignore a11y_no_noninteractive_tabindex (Exact amounts must be keyboard scrollable.) -->
                  <div
                    class="declared-value declared-amount"
                    role="region"
                    aria-label={`${row.label} in GBP`}
                    tabindex="0"
                    title={row.total.incomplete
                      ? "Known GBP amounts only. Some amounts unavailable."
                      : undefined}
                  >
                    {formatPounds(row.total.amount)}
                  </div>
                {:else}
                  <span class="declared-value declared-unavailable"
                    >{row.total.declarationCount === 0
                      ? "No declaration data available"
                      : formatPounds(row.total.amount)}</span
                  >
                {/if}
              </td>
            </tr>
          {/each}
        </DataTable>
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

<style>
  .member-declared-totals {
    grid-column: 1 / -1;
    min-width: 0;
  }
  .declared-total {
    text-align: right;
    width: 55%;
  }
  .declared-amount {
    font-size: var(--text-heading);
    color: var(--accent);
    font-variant-numeric: tabular-nums;
    overflow-x: auto;
    white-space: nowrap;
    max-width: 100%;
  }
  .declared-amount:focus-visible {
    outline: var(--focus-width) solid var(--focus-color);
    outline-offset: var(--focus-offset);
  }
  .declared-unavailable {
    font-size: var(--text-body);
    color: var(--muted);
  }
</style>
