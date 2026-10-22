<script lang="ts">
  import { DeclarationList, DeclarationCard, ValueRow } from "./lib/ui";
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

<DeclarationList>
  {#each declarations as source}
    {@const declaration = source.declaration}
    <DeclarationCard>
      {#snippet heading()}
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
      {/snippet}{#snippet registered()}
        {#if declaration.registeredAt}<time datetime={declaration.registeredAt}
            >{registrationDate(declaration.registeredAt)}</time
          >{:else}Registration date unavailable{/if}
      {/snippet}
      {#if declaration.entries.length === 0}<p class="empty-entry">
          The declaration included no monetary remuneration.
        </p>
      {:else}
        <ul class="funding-list">
          {#each declaration.entries as entry}
            <ValueRow>
              {#snippet source()}
                <span class="payment-type"
                  >{entry.paymentType || "Payment type unavailable"}</span
                >
              {/snippet}{#snippet value()}
                <span class="amount"
                  >{entry.currency === "GBP"
                    ? formatPounds(entry.amount)
                    : formatAmount(entry.amount)}</span
                >
              {/snippet}
            </ValueRow>
          {/each}
        </ul>
      {/if}
    </DeclarationCard>
  {/each}
</DeclarationList>

<style>
  a.declaration-reference {
    color: inherit;
  }
</style>
