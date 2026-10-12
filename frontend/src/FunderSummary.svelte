<script lang="ts">
  import { formatPounds } from "./lib/member-summary";
  import { memberHref, type FunderDetails } from "./lib/entity-details";
  import { partyChart } from "./lib/funder-summary";
  let { details }: { details: FunderDetails } = $props();
  const pounds = $derived(
    details.currencies.find((group) => group.currency === "GBP"),
  );
  const slices = $derived(partyChart(pounds?.parties ?? []));
</script>

<section aria-labelledby="party-heading">
  <div class="detail-section-heading">
    <h2 id="party-heading">Funding by party</h2>
    <p>Grouped by recipients' latest recorded party.</p>
  </div>
  {#if slices.length === 0}<p class="empty-details">
      {details.entryCount === 0
        ? "No funding entries are stored for this funder."
        : "No known funding totals are available."}
    </p>{:else}
    <div class="funder-party-chart">
      <div class="funder-pie">
        <svg
          viewBox="0 0 200 200"
          role="img"
          aria-label="Funding by party. Positive totals determine the pie shares."
        >
          {#if slices.every((slice) => slice.share === 0)}<circle
              cx="100"
              cy="100"
              r="90"
              fill="#ece8e1"
            />{/if}
          {#each slices as slice}
            {#if slice.share === 1}<circle
                cx="100"
                cy="100"
                r="90"
                fill={slice.color}
                ><title
                  >{slice.party.partyName}: {formatPounds(
                    slice.party.amount,
                  )}</title
                ></circle
              >
            {:else if slice.share > 0}<path d={slice.path} fill={slice.color}
                ><title
                  >{slice.party.partyName}: {formatPounds(
                    slice.party.amount,
                  )}</title
                ></path
              >{/if}
          {/each}
        </svg>
        {#if slices.every((slice) => slice.share === 0)}<p
            class="coverage-note"
          >
            Pie unavailable. No positive known totals are recorded.
          </p>{:else if slices.some((slice) => slice.party.amount === null || slice.party.amount.startsWith("-"))}<p
            class="filter-context"
          >
            The pie shows positive known totals. Negative and unavailable totals
            remain in the legend.
          </p>{/if}
      </div>
      <ul class="funder-party-legend">
        {#each slices as slice}<li>
            <span class="party-swatch" style:background={slice.color}></span>
            <div>
              <span>{slice.party.partyName || "Party unavailable"}</span><span
                class="entry-count"
                >{slice.party.entryCount}
                {slice.party.entryCount === 1
                  ? "entry"
                  : "entries"}{#if slice.party.unknownAmountCount > 0}, {slice
                    .party.unknownAmountCount}
                  {slice.party.unknownAmountCount === 1 ? "amount" : "amounts"} unavailable{/if}</span
              >
            </div>
            <strong>{formatPounds(slice.party.amount)}</strong>
          </li>{/each}
      </ul>
    </div>
  {/if}
</section>
<section aria-labelledby="recipients-heading">
  <div class="detail-section-heading">
    <h2 id="recipients-heading">Top 10 recipients</h2>
    <p>MPs ranked by known declared value.</p>
  </div>
  {#if !pounds?.topRecipients.length}<p class="empty-details">
      No recipients are available.
    </p>{:else}
    <div class="summary-columns funder-recipients">
      <table>
        <caption class="visually-hidden"
          >Top recipients of declared funding</caption
        ><thead
          ><tr><th scope="col">MP</th><th scope="col">Known total</th></tr
          ></thead
        ><tbody>
          {#each pounds.topRecipients as recipient}<tr
              ><th scope="row"
                ><a href={memberHref(recipient.member.id, "summary")}
                  >{recipient.member.name}</a
                ><span class="entry-count"
                  >{recipient.member.partyName || "Party unavailable"}</span
                ></th
              ><td
                ><span class="amount">{formatPounds(recipient.amount)}</span
                >{#if recipient.unknownAmountCount > 0}<span
                    class="partial-note"
                    >{recipient.unknownAmountCount}
                    {recipient.unknownAmountCount === 1 ? "amount" : "amounts"} unavailable</span
                  >{/if}</td
              ></tr
            >{/each}
        </tbody>
      </table>
    </div>
  {/if}
</section>
{#if details.funder.aliases.length > 0}<details
    class="recorded-names funder-recorded-names"
    role="region"
    aria-label="Recorded names"
  >
    <summary>Recorded names</summary>
    <ul>
      {#each details.funder.aliases as alias}<li>{alias}</li>{/each}
    </ul>
  </details>{/if}
