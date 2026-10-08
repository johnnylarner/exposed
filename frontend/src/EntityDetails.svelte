<script lang="ts">
  import Icon from "./lib/Icon.svelte";
  import { entityHref, funderKindLabel } from "./lib/search";
  import {
    backToSearch,
    formatAmount,
    loadDetails,
    registrationDate,
    type DetailRoute,
    type DetailState,
  } from "./lib/entity-details";
  let { route }: { route: Exclude<DetailRoute, { kind: "search" }> } = $props();
  let attempt = $state(0);
  let detailState = $state<DetailState>({ kind: "loading" });
  const backHref = backToSearch();
  $effect(() => {
    const currentRoute = route;
    void attempt;
    if (currentRoute.kind === "invalid") {
      detailState = { kind: "not-found" };
      return;
    }
    const controller = new AbortController();
    let current = true;
    detailState = { kind: "loading" };
    void loadDetails(currentRoute, controller.signal)
      .then((result) => {
        if (current) detailState = result;
      })
      .catch(() => {});
    return () => {
      current = false;
      controller.abort();
    };
  });
</script>

<main
  id="details"
  class="detail-page"
  class:funder-page={route.kind === "funder"}
  aria-busy={detailState.kind === "loading"}
>
  <a class="back-link" href={backHref}>Back to search</a>
  {#if detailState.kind === "loading"}
    <div class="detail-message" role="status">
      <span class="loading-indicator" aria-hidden="true"></span>
      <h1>Loading details</h1>
    </div>
  {:else if detailState.kind === "not-found"}
    <div class="detail-message">
      <h1>Entity not found</h1>
      <p>
        This MP or funder is not in the stored register. Search for another
        name.
      </p>
      <a class="text-button" href={backHref}>Search MPs and funders</a>
    </div>
  {:else if detailState.kind === "error"}
    <div class="detail-message" role="alert">
      <h1>Details couldn’t load</h1>
      <p>{detailState.message}</p>
      <button class="retry-button" onclick={() => (attempt += 1)}
        ><Icon name="retry" size={16} />Try again</button
      >
    </div>
  {:else if detailState.kind === "member"}
    {@const details = detailState.details}
    <header class="profile-heading">
      <span class="profile-type"><Icon name="parliament" size={22} />MP</span>
      <h1>{details.member.name}</h1>
      <p class="profile-meta">
        {details.member.partyName || "Party unavailable"}
      </p>
      <dl class="profile-facts">
        <div>
          <dt>Latest membership</dt>
          <dd>{details.member.membershipFrom || "Unavailable"}</dd>
        </div>
        <div>
          <dt>Commons membership</dt>
          <dd>
            {details.member.isCurrentCommons ? "Current MP" : "Former MP"}
          </dd>
        </div>
      </dl>
    </header>
    <section aria-labelledby="declarations-heading">
      <div class="detail-section-heading">
        <h2 id="declarations-heading">Recent declarations</h2>
        <p>
          Showing {details.declarations.length} of {details.declarationCount}
          stored {details.declarationCount === 1
            ? "declaration"
            : "declarations"}, most recent first.
        </p>
      </div>
      {#if details.declarations.length === 0}
        <p class="empty-details">No declarations are stored for this MP.</p>
      {:else}
        <ol class="declarations-list">
          {#each details.declarations as declaration}
            <li class="declaration">
              <header class="declaration-heading">
                <div>
                  <h3>{declaration.categoryName}</h3>
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
                          >{entry.paymentType ||
                            "Payment type unavailable"}</span
                        >
                      </div>
                      <div class="funding-value">
                        <span class="amount">{formatAmount(entry.amount)}</span
                        ><span class="currency-label"
                          >{entry.currency?.trim() ||
                            "Currency unavailable"}</span
                        >
                      </div>
                    </li>
                  {/each}
                </ul>
              {/if}
            </li>
          {/each}
        </ol>
      {/if}
    </section>
  {:else}
    {@const details = detailState.details}
    <header class="profile-heading">
      <span class="profile-type"><Icon name="coin" size={22} />Funder</span>
      <h1>{details.funder.name}</h1>
      <p class="profile-meta">{funderKindLabel(details.funder.funderKind)}</p>
      <dl class="profile-facts">
        {#if details.funder.companyNumber}<div>
            <dt>Company number</dt>
            <dd>{details.funder.companyNumber}</dd>
          </div>{/if}
        <div>
          <dt>Recorded support</dt>
          <dd>
            {details.entryCount}
            {details.entryCount === 1 ? "funding entry" : "funding entries"}
          </dd>
        </div>
      </dl>
    </header>
    <p class="summary-context">
      Declared support is grouped by recipients’ latest stored party. It may
      include payments or benefits beyond direct donations. Parties do not
      describe membership when support was received. This page uses the funder’s
      exact source name.
    </p>
    {#if details.unknownCurrencyCount > 0}<p class="coverage-note">
        {details.unknownCurrencyCount}
        {details.unknownCurrencyCount === 1 ? "entry has" : "entries have"} an unknown
        currency and {details.unknownCurrencyCount === 1 ? "is" : "are"} excluded
        from totals and rankings.
      </p>{/if}
    {#if details.currencies.length === 0}<p class="empty-details">
        {details.entryCount === 0
          ? "No funding entries are stored for this funder."
          : "No known currencies are available for totals or rankings."}
      </p>
    {:else}
      {#each details.currencies as group}
        <section
          class="currency-section"
          aria-label={`${group.currency} declared support`}
        >
          <h2>{group.currency}</h2>
          <div class="summary-columns">
            <div>
              <h3>Declared support by party</h3>
              <table>
                <caption class="visually-hidden"
                  >{group.currency} declared support by recipients’ latest stored
                  party</caption
                ><thead
                  ><tr
                    ><th scope="col">Party</th><th scope="col">Known total</th
                    ></tr
                  ></thead
                ><tbody
                  >{#each group.parties as party}<tr
                      ><th scope="row"
                        >{party.partyName || "Party unavailable"}<span
                          class="entry-count"
                          >{party.entryCount}
                          {party.entryCount === 1 ? "entry" : "entries"}</span
                        ></th
                      ><td
                        ><span class="amount">{formatAmount(party.amount)}</span
                        >{#if party.unknownAmountCount > 0}<span
                            class="partial-note"
                            >{party.unknownAmountCount}
                            {party.unknownAmountCount === 1
                              ? "amount"
                              : "amounts"} unavailable</span
                          >{/if}</td
                      ></tr
                    >{/each}</tbody
                >
              </table>
            </div>
            <div>
              <h3>Top recipients</h3>
              <p class="ranking-note">
                Up to {details.recipientLimit} MPs, ranked by known {group.currency}
                total.
              </p>
              <table>
                <caption class="visually-hidden"
                  >Top recipients of {group.currency} declared support</caption
                ><thead
                  ><tr
                    ><th scope="col">MP</th><th scope="col">Known total</th></tr
                  ></thead
                ><tbody
                  >{#each group.topRecipients as recipient}<tr
                      ><th scope="row"
                        ><a
                          href={entityHref({
                            kind: "MP",
                            id: recipient.member.id,
                          })}>{recipient.member.name}</a
                        ><span class="entry-count"
                          >{recipient.member.partyName ||
                            "Party unavailable"}</span
                        ></th
                      ><td
                        ><span class="amount"
                          >{formatAmount(recipient.amount)}</span
                        >{#if recipient.unknownAmountCount > 0}<span
                            class="partial-note"
                            >{recipient.unknownAmountCount}
                            {recipient.unknownAmountCount === 1
                              ? "amount"
                              : "amounts"} unavailable</span
                          >{/if}</td
                      ></tr
                    >{/each}</tbody
                >
              </table>
            </div>
          </div>
        </section>
      {/each}
    {/if}
    <p class="detail-footnote">
      Currencies are shown separately without conversion. Known totals include
      zero and negative declared values. Unavailable amounts are not treated as
      zero.
    </p>
  {/if}
</main>
