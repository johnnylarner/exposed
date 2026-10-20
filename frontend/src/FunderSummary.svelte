<script lang="ts">
  import { page } from "$app/state";
  import { onDestroy } from "svelte";
  import { formatPounds } from "./lib/member-summary";
  import { memberHref, type FunderDetails } from "./lib/entity-details";
  import { partyChart } from "./lib/funder-summary";
  let { details }: { details: FunderDetails } = $props();
  const pounds = $derived(
    details.currencies.find((group) => group.currency === "GBP"),
  );
  const slices = $derived(partyChart(pounds?.parties ?? []));
  const recipients = $derived(pounds?.topRecipients.slice(0, 5) ?? []);
  let tooltip = $state<{ index: number; left: number; top: number } | null>(
    null,
  );
  let hoverDismissed = false;
  let anchor: Element | null = null;
  let anchorTop = 0;
  let dismissal: ReturnType<typeof setTimeout> | undefined;
  function keepOpen() {
    clearTimeout(dismissal);
  }
  function show(index: number, target: EventTarget | null) {
    if (!(target instanceof Element)) return;
    keepOpen();
    anchor = target.closest("svg") ?? target;
    const bounds = anchor.getBoundingClientRect();
    anchorTop = bounds.top;
    tooltip = {
      index,
      left: Math.max(12, Math.min(bounds.left, window.innerWidth - 292)),
      top:
        bounds.bottom + 100 > window.innerHeight
          ? Math.max(12, bounds.top - 100)
          : bounds.bottom + 8,
    };
  }
  function hide() {
    keepOpen();
    tooltip = null;
  }
  function dismissIfAnchorMoved() {
    if (anchor?.getBoundingClientRect().top !== anchorTop) hide();
  }
  onDestroy(keepOpen);
  function dismiss() {
    dismissal = setTimeout(() => {
      tooltip = null;
    }, 120);
  }
  function escape(event: KeyboardEvent) {
    if (event.key === "Escape") {
      hoverDismissed = true;
      keepOpen();
      tooltip = null;
    }
  }
</script>

<svelte:window
  onkeydown={escape}
  onscroll={dismissIfAnchorMoved}
  onresize={hide}
/>
<div class="member-summary-top funder-summary-top">
  <section aria-labelledby="party-heading">
    <div class="detail-section-heading">
      <h2 id="party-heading">Funding by party</h2>
      <p>Grouped by recipients' latest recorded party.</p>
    </div>
    <div class="funder-party-chart">
      {#if slices.length === 0}<p class="empty-details funder-party-empty">
          {details.entryCount === 0
            ? "No funding entries are stored for this funder."
            : "No known funding totals are available."}
        </p>{:else}
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
                fill="var(--chart-track)"
              />{/if}
            {#each slices as slice, index}
              {#if slice.share > 0}
                <g
                  role="img"
                  aria-label={`${slice.party.partyName || "Party unavailable"}: ${formatPounds(slice.party.amount)}`}
                  aria-describedby={tooltip?.index === index
                    ? "party-tooltip"
                    : undefined}
                  onpointermove={(event) => {
                    if (
                      hoverDismissed &&
                      (event.movementX || event.movementY) &&
                      event.pointerType !== "touch"
                    ) {
                      hoverDismissed = false;
                      show(index, event.currentTarget);
                    }
                  }}
                  onpointerenter={(event) => {
                    if (event.pointerType !== "touch" && !hoverDismissed)
                      show(index, event.currentTarget);
                  }}
                  onpointerleave={dismiss}
                >
                  {#if slice.share === 1}<circle
                      cx="100"
                      cy="100"
                      r="90"
                      fill={slice.color}
                    />
                  {:else}<path d={slice.path} fill={slice.color} />{/if}
                </g>
              {/if}
            {/each}
          </svg>
        </div>
        <ul class="funder-party-legend" onscroll={dismissIfAnchorMoved}>
          {#each slices as slice, index}<li
              aria-label={`${slice.party.partyName || "Party unavailable"}: ${formatPounds(slice.party.amount)}`}
              aria-describedby={tooltip?.index === index
                ? "party-tooltip"
                : undefined}
              onpointermove={(event) => {
                if (
                  hoverDismissed &&
                  (event.movementX || event.movementY) &&
                  event.pointerType !== "touch"
                ) {
                  hoverDismissed = false;
                  show(index, event.currentTarget);
                }
              }}
              onpointerenter={(event) => {
                if (event.pointerType !== "touch" && !hoverDismissed)
                  show(index, event.currentTarget);
              }}
              onpointerleave={dismiss}
            >
              <span class="party-swatch" style:background={slice.color}></span>
              <div>
                <span class="party-name"
                  >{slice.party.partyName || "Party unavailable"}</span
                ><span class="entry-count"
                  >{slice.party.entryCount}
                  {slice.party.entryCount === 1
                    ? "entry"
                    : "entries"}{#if slice.party.unknownAmountCount > 0}, {slice
                      .party.unknownAmountCount}
                    {slice.party.unknownAmountCount === 1
                      ? "amount"
                      : "amounts"} unavailable{/if}</span
                >
              </div>
            </li>{/each}
        </ul>
      {/if}
    </div>
    {#if slices.length > 0}
      {#if slices.every((slice) => slice.share === 0)}<p class="coverage-note">
          Pie unavailable. No positive known totals are recorded.
        </p>{:else if slices.some((slice) => slice.party.amount === null || slice.party.amount.startsWith("-"))}<p
          class="filter-context"
        >
          The pie shows positive known totals. Negative and unavailable totals
          are available in the party legend.
        </p>{/if}
    {/if}
  </section>
  <section class="funder-recipient-panel" aria-labelledby="recipients-heading">
    <div class="detail-section-heading">
      <h2 id="recipients-heading">Top 5 recipients</h2>
      <p>MPs ranked by known declared value.</p>
    </div>
    {#if !recipients.length}<p class="empty-details">
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
            {#each recipients as recipient}<tr
                ><th scope="row"
                  ><a
                    href={memberHref(
                      recipient.member.id,
                      "summary",
                      page.url.searchParams,
                    )}>{recipient.member.name}</a
                  ><span class="entry-count"
                    >{recipient.member.partyName || "Party unavailable"}</span
                  ></th
                ><td
                  ><span class="amount">{formatPounds(recipient.amount)}</span
                  >{#if recipient.unknownAmountCount > 0}<span
                      class="partial-note"
                      >{recipient.unknownAmountCount}
                      {recipient.unknownAmountCount === 1
                        ? "amount"
                        : "amounts"} unavailable</span
                    >{/if}</td
                ></tr
              >{/each}
          </tbody>
        </table>
      </div>
    {/if}
  </section>
</div>
{#if tooltip}{@const party = slices[tooltip.index].party}
  <div
    id="party-tooltip"
    class="party-tooltip"
    role="tooltip"
    style:left={`${tooltip.left}px`}
    style:top={`${tooltip.top}px`}
    onpointerenter={keepOpen}
    onpointerleave={dismiss}
  >
    <span>{party.partyName || "Party unavailable"}</span>
    <strong>{formatPounds(party.amount)}</strong>
    <span class="entry-count"
      >{party.entryCount}
      {party.entryCount === 1
        ? "entry"
        : "entries"}{#if party.unknownAmountCount > 0}, {party.unknownAmountCount}
        {party.unknownAmountCount === 1 ? "amount" : "amounts"} unavailable{/if}</span
    >
  </div>
{/if}
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
