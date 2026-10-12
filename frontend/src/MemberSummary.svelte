<script lang="ts">
  import MemberDeclarations from "./MemberDeclarations.svelte";
  import MemberEarningsChart from "./MemberEarningsChart.svelte";
  import {
    categoryBars,
    memberHref,
    type MemberDetails,
  } from "./lib/entity-details";
  import { formatPounds, summarizeMember } from "./lib/member-summary";
  let {
    details,
    summary,
  }: { details: MemberDetails; summary: ReturnType<typeof summarizeMember> } =
    $props();
  const bars = $derived(categoryBars(summary.categories, "total"));
  const baseline = $derived(
    bars.reduce((total, bar) => total + (bar.negative ? bar.width : 0), 0),
  );
</script>

<div class="member-summary-top">
  <section class="member-breakdown" aria-labelledby="breakdown-heading">
    <h2 id="breakdown-heading">Declared value by category</h2>
    {#if summary.categories.length}
      <ul class="category-list" style:--baseline={`${baseline}%`}>
        {#each summary.categories as category, index}
          <li>
            <div class="category-label">
              <span>{category.categoryName}</span><span class="amount"
                >{formatPounds(category.amount)}</span
              >
            </div>
            <div class="signed-bar" aria-hidden="true">
              <span
                class:negative={bars[index].negative}
                style:width={`${bars[index].width}%`}
              ></span>
            </div>
            {#if category.unknownAmountCount}<p class="partial-note">
                Some amounts unavailable
              </p>{/if}
          </li>
        {/each}
      </ul>
    {:else}<p class="empty-details">No known declared values available.</p>{/if}
  </section>
  <MemberEarningsChart weeks={summary.earnings} undated={summary.undated} />
</div>
<section aria-labelledby="recent-declarations-heading">
  <div class="detail-section-heading">
    <h2 id="recent-declarations-heading">Recent declarations</h2>
  </div>
  {#if details.declarations.length}<MemberDeclarations
      declarations={details.declarations.slice(0, 5)}
    />{:else}<p class="empty-details">
      No declarations are stored for this MP.
    </p>{/if}
  <a
    class="text-button history-link"
    href={memberHref(details.member.id, "history")}>View declaration history</a
  >
</section>
