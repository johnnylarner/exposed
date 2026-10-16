<script lang="ts">
  import { page } from "$app/state";
  import { formatPounds } from "./lib/member-summary";
  import { entityHref } from "./lib/search";
  import {
    formatAmount,
    registrationDate,
    type MemberDeclaration,
  } from "./lib/entity-details";
  let { declarations }: { declarations: MemberDeclaration[] } = $props();
</script>

<ol class="declarations-list member-declarations">
  {#each declarations as declaration}
    <li class="declaration">
      <header class="declaration-heading">
        <div>
          <h3>{declaration.categoryName}</h3>
          <a
            class="declaration-reference"
            href={`https://interests-api.parliament.uk/api/v2/Interests/${declaration.sourceId}`}
            target="_blank"
            rel="noreferrer">Declaration {declaration.sourceId}</a
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
          The declaration included no monetary remuneration.
        </p>
      {:else}
        <ul class="funding-list">
          {#each declaration.entries as entry}
            <li>
              <div class="funding-source">
                {#if entry.funder}<a
                    href={entityHref(
                      {
                        kind: "Funder",
                        id: entry.funder.id,
                      },
                      page.url.searchParams,
                    )}>{entry.funder.name}</a
                  >{:else}<span>Funder unavailable</span>{/if}<span
                  class="payment-type"
                  >{entry.paymentType || "Payment type unavailable"}</span
                >
              </div>
              <div class="funding-value">
                <span class="amount"
                  >{entry.currency === "GBP"
                    ? formatPounds(entry.amount)
                    : formatAmount(entry.amount)}</span
                >{#if entry.currency?.trim() && entry.currency !== "GBP"}<span
                    class="currency-label">{entry.currency}</span
                  >{/if}
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </li>
  {/each}
</ol>
