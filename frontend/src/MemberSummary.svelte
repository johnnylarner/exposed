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
      Earnings, gifts and support across {details.declarations.length} stored declarations.
      Currencies shown separately.
    </p>
    {#if details.summary.firstRegisteredAt && details.summary.lastRegisteredAt}<p
        class="stored-coverage"
      >
        Registered {registrationDate(details.summary.firstRegisteredAt)} to {registrationDate(
          details.summary.lastRegisteredAt,
        )}.
        {#if details.summary.undatedDeclarationCount}
          {details.summary.undatedDeclarationCount}
          {details.summary.undatedDeclarationCount === 1
            ? "declaration has"
            : "declarations have"} no registration date.
        {/if}
      </p>{/if}
    {#if details.summary.currencies.length === 0}<p class="empty-details">
        {details.summary.entryCount === 0
          ? "No funding entries are stored for this MP."
          : "No known currencies are available for totals."}
      </p>{/if}
    {#if details.summary.undatedDeclarationCount && !details.summary.firstRegisteredAt}<p
        class="stored-coverage"
      >
        Registration dates are unavailable.
      </p>{/if}
    <div class="member-currency-ledger">
      {#each details.summary.currencies as group}
        {@const bars = categoryBars(group.categories)}
        {@const signed = bars.some((bar) => bar.negative)}
        <section
          class="category-currency"
          aria-label={`${group.currency} declared value`}
        >
          <div class="currency-heading">
            <h3>{group.currency}</h3>
            <div class="currency-total">
              <p class="member-total">{formatAmount(group.amount)}</p>
              <p class="entry-count">
                {group.entryCount} funding {group.entryCount === 1
                  ? "entry"
                  : "entries"}{#if group.unknownAmountCount}, {group.unknownAmountCount}
                  {group.unknownAmountCount === 1 ? "amount" : "amounts"} unavailable{/if}
              </p>
            </div>
          </div>
          <ul
            class="category-list"
            aria-label={`Declared value by category in ${group.currency}`}
          >
            {#each group.categories as category, index}<li>
                <div class="category-label">
                  <span>{category.categoryName}</span><span class="amount"
                    >{formatAmount(category.amount)} {group.currency}</span
                  >
                </div>
                {#if category.amount !== null}<div
                    class="signed-bar"
                    class:signed
                    aria-hidden="true"
                  >
                    <span
                      class:negative={bars[index].negative}
                      style:width={`${bars[index].width / (signed ? 2 : 1)}%`}
                    ></span>
                  </div>{/if}
                {#if category.unknownAmountCount}<p class="partial-note">
                    {category.unknownAmountCount}
                    {category.unknownAmountCount === 1 ? "amount" : "amounts"} unavailable
                  </p>{/if}
              </li>{/each}
          </ul>
          {#if signed}<p class="bar-baseline-note">
              Bars extend left or right from zero.
            </p>{/if}
        </section>
      {/each}
    </div>
    {#if details.summary.unknownCurrencyCount}<p class="coverage-note">
        {details.summary.unknownCurrencyCount} funding {details.summary
          .unknownCurrencyCount === 1
          ? "entry has"
          : "entries have"} an unknown currency and {details.summary
          .unknownCurrencyCount === 1
          ? "is"
          : "are"} excluded from monetary totals.
      </p>{/if}
  </section>
</div>
<section aria-labelledby="support-heading">
  <div class="detail-section-heading">
    <h2 id="support-heading">Recent donations and support</h2>
    <p>
      Includes loans and support received by the MP or a party organisation.
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
