<script lang="ts">
  import MemberDeclarations from "./MemberDeclarations.svelte";
  import {
    categoryBars,
    formatAmount,
    memberHref,
    registrationDate,
    type MemberDetails,
  } from "./lib/entity-details";
  let { details }: { details: MemberDetails } = $props();
</script>

<div class="member-summary-top">
  <section class="member-overview" aria-labelledby="overview-heading">
    <h2 id="overview-heading">Known declared value</h2>
    <p class="summary-context">
      Across {details.declarations.length} stored declarations. Includes earnings,
      gifts and support.
    </p>
    {#if details.summary.firstRegisteredAt && details.summary.lastRegisteredAt}<p
        class="stored-coverage"
      >
        Registered {registrationDate(details.summary.firstRegisteredAt)} to {registrationDate(
          details.summary.lastRegisteredAt,
        )}.{#if details.summary.undatedDeclarationCount}
          {details.summary.undatedDeclarationCount} declarations have no registration
          date.{/if}
      </p>{/if}
    {#if details.summary.currencies.length === 0}<p class="empty-details">
        {details.summary.entryCount === 0
          ? "No funding entries are stored for this MP."
          : "No known currencies are available for totals."}
      </p>{/if}
    <div class="member-currency-totals">
      {#each details.summary.currencies as group}<div>
          <h3>{group.currency}</h3>
          <p class="member-total">{formatAmount(group.amount)}</p>
          <p class="entry-count">
            {group.entryCount} funding {group.entryCount === 1
              ? "entry"
              : "entries"}{#if group.unknownAmountCount}, {group.unknownAmountCount}
              amounts unavailable{/if}
          </p>
        </div>{/each}
    </div>
    {#if details.summary.undatedDeclarationCount && !details.summary.firstRegisteredAt}<p
        class="stored-coverage"
      >
        Registration dates are unavailable.
      </p>{/if}
    {#if details.summary.unknownCurrencyCount}<p class="coverage-note">
        {details.summary.unknownCurrencyCount} funding {details.summary
          .unknownCurrencyCount === 1
          ? "entry has"
          : "entries have"} an unknown currency and are excluded from monetary totals.
      </p>{/if}
  </section>
  <section class="member-breakdown" aria-labelledby="breakdown-heading">
    <h2 id="breakdown-heading">Declared value by category</h2>
    <p class="summary-context">
      Amounts recorded in each category, with currencies kept separate.
    </p>
    {#each details.summary.currencies as group}
      {@const bars = categoryBars(group.categories)}
      {@const signed = bars.some((bar) => bar.negative)}
      <div class="category-currency">
        <h3>{group.currency}</h3>
        <ul class="category-list">
          {#each group.categories as category, index}<li>
              <div class="category-label">
                <span>{category.categoryName}</span><span class="amount"
                  >{formatAmount(category.amount)} {group.currency}</span
                >
              </div>
              <div class="signed-bar" class:signed aria-hidden="true">
                <span
                  class:negative={bars[index].negative}
                  style:width={`${bars[index].width / (signed ? 2 : 1)}%`}
                ></span>
              </div>
              {#if category.unknownAmountCount}<p class="partial-note">
                  {category.unknownAmountCount} amounts unavailable
                </p>{/if}
            </li>{/each}
        </ul>
      </div>
    {/each}
  </section>
</div>
<section aria-labelledby="support-heading">
  <div class="detail-section-heading">
    <h2 id="support-heading">Recent donations and support</h2>
    <p>
      Latest declared support, including loans. Support may be received by the
      MP or a party organisation.
    </p>
  </div>
  {#if details.recentSupport.length}<MemberDeclarations
      declarations={details.recentSupport}
      showCategory={false}
    />{:else}<p class="empty-details">
      No donations or support declarations are stored for this MP.
    </p>{/if}
  <a
    class="text-button history-link"
    href={memberHref(details.member.id, "history")}
    >View all {details.declarations.length} declarations</a
  >
</section>
