<script lang="ts">
  import { entityHref } from "./lib/search";
  import {
    formatAmount,
    registrationDate,
    type MemberDeclaration,
  } from "./lib/entity-details";
  let {
    declarations,
    showCategory = true,
  }: { declarations: MemberDeclaration[]; showCategory?: boolean } = $props();
</script>

<ol class="declarations-list member-declarations">
  {#each declarations as declaration}
    <li class="declaration">
      <header class="declaration-heading">
        <div>
          {#if showCategory}<h3>{declaration.categoryName}</h3>{/if}
          <span class="declaration-reference"
            >Declaration {declaration.sourceId}</span
          >
        </div>
        <p>
          {#if declaration.registeredAt}<time
              datetime={declaration.registeredAt}
              >{registrationDate(declaration.registeredAt)}</time
            >{:else}Registration date unavailable{/if}
        </p>
      </header>
      {#if declaration.entries.length === 0}<p class="empty-entry">
          No funding entries recorded.
        </p>
      {:else}
        <ul class="funding-list">
          {#each declaration.entries as entry}
            <li>
              <div class="funding-source">
                {#if entry.funder}<a
                    href={entityHref({
                      kind: "Funder",
                      id: entry.funder.id,
                    })}>{entry.funder.name}</a
                  >{:else}<span>Funder unavailable</span>{/if}<span
                  class="payment-type"
                  >{entry.paymentType || "Payment type unavailable"}</span
                >
              </div>
              <div class="funding-value">
                <span class="amount">{formatAmount(entry.amount)}</span><span
                  class="currency-label"
                  >{entry.currency?.trim() || "Currency unavailable"}</span
                >
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </li>
  {/each}
</ol>
