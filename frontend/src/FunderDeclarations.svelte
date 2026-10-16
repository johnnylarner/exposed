<script lang="ts">
  import { page } from "$app/state";
  import { formatPounds } from "./lib/member-summary";
  import { entityHref } from "./lib/search";
  import {
    formatAmount,
    registrationDate,
    type FunderDeclaration,
  } from "./lib/entity-details";
  let { declarations }: { declarations: FunderDeclaration[] } = $props();
</script>

<ol class="declarations-list member-declarations">
  {#each declarations as source}
    {@const declaration = source.declaration}
    <li class="declaration">
      <header class="declaration-heading">
        <div>
          <h3>
            <a
              href={entityHref(
                { kind: "MP", id: source.member.id },
                page.url.searchParams,
              )}>{source.member.name}</a
            >
          </h3>
          <span class="entry-count">{source.member.partyName}</span>
          <p>{declaration.categoryName}</p>
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
                <span class="payment-type"
                  >{entry.paymentType || "Payment type unavailable"}</span
                >
              </div>
              <div class="funding-value">
                <span class="amount"
                  >{entry.currency === "GBP"
                    ? formatPounds(entry.amount)
                    : formatAmount(entry.amount)}</span
                >
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </li>
  {/each}
</ol>
