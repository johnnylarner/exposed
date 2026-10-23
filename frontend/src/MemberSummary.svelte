<script lang="ts">
  import { SignedBarList, ButtonLink, SectionHeading } from "./lib/ui";
  import { page } from "$app/state";
  import MemberDeclarations from "./MemberDeclarations.svelte";
  import MemberMoneySources from "./MemberMoneySources.svelte";
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
    <h2 class="section-title" id="breakdown-heading">
      Declared value by category
    </h2>
    {#if summary.categories.length}
      <SignedBarList
        variant="compact"
        {baseline}
        bars={summary.categories.map((category, index) => ({
          label: category.categoryName,
          value: formatPounds(category.amount),
          width: bars[index].width,
          negative: bars[index].negative,
          note: category.unknownAmountCount
            ? "Some amounts unavailable"
            : undefined,
        }))}
      />
    {:else}<p class="empty-details">No known declared values available.</p>{/if}
  </section>
  <MemberMoneySources sources={summary.sources} />
</div>
<section aria-labelledby="recent-declarations-heading">
  <SectionHeading id="recent-declarations-heading" title="Recent declarations"
  ></SectionHeading>
  {#if details.declarations.length}<MemberDeclarations
      declarations={details.declarations.slice(0, 5)}
    />{:else}<p class="empty-details">
      No declarations are stored for this MP.
    </p>{/if}
  <ButtonLink
    variant="quiet"
    size="inline"
    class="history-link"
    href={memberHref(details.member.id, "history", page.url.searchParams)}
    >View declaration history</ButtonLink
  >
</section>
