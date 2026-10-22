<script lang="ts">
  import { onMount } from "svelte";
  import AppShell from "../../AppShell.svelte";
  import { Button, ButtonLink, Field, Notice } from "../../lib/ui";
  const swatches = [
    { name: "Canvas", token: "--canvas" },
    { name: "Surface", token: "--surface" },
    { name: "Text", token: "--text" },
    { name: "Muted text", token: "--muted" },
    { name: "Members", token: "--accent-member" },
    { name: "Funders", token: "--accent-funder" },
  ];
  let colors = $state<Record<string, string>>({});
  onMount(() => {
    const styles = getComputedStyle(document.documentElement);
    colors = Object.fromEntries(
      swatches.map(({ token }) => [
        token,
        styles.getPropertyValue(token).trim(),
      ]),
    );
  });
  let query = $state("");
  let from = $state("");
  let to = $state("");
  let submitted = $state(false);
  let status = $state("No filters applied.");
  const reversed = $derived(from !== "" && to !== "" && from > to);
  const error = $derived(
    submitted && reversed
      ? "Choose an end date on or after the start date."
      : undefined,
  );
  function apply(event: SubmitEvent) {
    event.preventDefault();
    submitted = true;
    if (!reversed)
      status = `Filters applied${query ? ` for ${query}` : ""}. ${from || "Any start date"} to ${to || "any end date"}.`;
  }
  function clear() {
    query = "";
    from = "";
    to = "";
    submitted = false;
    status = "No filters applied.";
  }
</script>

<svelte:head
  ><title>Design system · Exposed</title><meta
    name="description"
    content="The foundations and accessible controls used to explore parliamentary declarations on Exposed."
  /></svelte:head
>
<AppShell target="design-system" current="design-system">
  <main id="design-system" class="reference" tabindex="-1">
    <header class="intro">
      <h1>Design system</h1>
      <p>
        Clear controls for exploring public interests. A working reference for
        the interface behind Exposed's parliamentary data.
      </p>
      <nav aria-label="Design system sections">
        <a href="#foundations">Foundations</a><a href="#type">Typography</a><a
          href="#actions">Actions</a
        ><a href="#fields">Fields</a><a href="#feedback">Feedback</a>
      </nav>
    </header>
    <section id="foundations" aria-labelledby="foundations-heading">
      <h2 id="foundations-heading">Foundations</h2>
      <p>
        Black keeps the focus on declarations. Purple connects member views;
        teal connects funder views.
      </p>
      <div class="swatches">
        {#each swatches as swatch}<div class="swatch">
            <span class="color" style:background={`var(${swatch.token})`}
            ></span><strong>{swatch.name}</strong><code>{swatch.token}</code
            ><span>{colors[swatch.token] ?? ""}</span>
          </div>{/each}
      </div>
      <div class="foundation-notes">
        <p>
          Spacing follows a 4px base, with 8, 12, 16, 24 and 32px steps.
          Controls share a 6px radius and a visible accent focus ring.
        </p>
        <p>
          Color supports meaning. Labels, shapes and written feedback must carry
          it too.
        </p>
      </div>
    </section>
    <section id="type" aria-labelledby="type-heading">
      <h2 id="type-heading">Typography</h2>
      <p>
        Manrope throughout. Short headings introduce the data; quieter context
        explains its limits.
      </p>
      <div class="type-samples">
        <div>
          <span class="sample-heading">Declaration history</span><code
            >Heading · 22px / 600</code
          >
        </div>
        <div>
          <span class="sample-body"
            >Registration dates include both ends of the selected period.</span
          ><code>Body · 14px / 1.8</code>
        </div>
        <div>
          <span class="sample-context"
            >Declarations without a registration date appear last.</span
          ><code>Context · 12px / 1.6</code>
        </div>
      </div>
    </section>
    <section id="actions" aria-labelledby="actions-heading">
      <h2 id="actions-heading">Actions</h2>
      <p>
        Buttons change the current view. Links take you to another page. Both
        retain their native keyboard behavior.
      </p>
      <div class="action-examples">
        <ButtonLink href="/search" variant="primary"
          >Search MPs and funders</ButtonLink
        ><ButtonLink href="/methodology">Read methodology</ButtonLink><Button
          disabled>Previous</Button
        >
      </div>
      <div class="funder-example">
        <span>Funder context</span>
        <div class="action-examples">
          <ButtonLink href="/search" variant="primary">Find a funder</ButtonLink
          ><Button
            variant="quiet"
            onclick={() => (status = "Funder example selected.")}
            >Show funder context</Button
          >
        </div>
      </div>
      <details>
        <summary>Using actions</summary>
        <pre><code
            >{'<Button onclick={clearDates} disabled={!from && !to}>Clear dates</Button>\n<ButtonLink href="/search" variant="primary">Search MPs and funders</ButtonLink>'}</code
          ></pre>
      </details>
    </section>
    <section id="fields" aria-labelledby="fields-heading">
      <h2 id="fields-heading">Fields</h2>
      <p>
        Every field has a visible label. Help and errors stay connected to the
        input. Try reversing the dates, then correct or clear them.
      </p>
      <form onsubmit={apply} onreset={clear} novalidate>
        <Field
          label="Member or funder"
          type="search"
          bind:value={query}
          hint="Try a name, such as Jane Smith."
          aria-describedby="demo-context"
        />
        <p id="demo-context" class="context">
          This example stays on this page and does not search parliamentary
          records.
        </p>
        <div class="date-fields">
          <Field label="Registered from" type="date" bind:value={from} /><Field
            label="Registered to"
            type="date"
            bind:value={to}
            hint="Both dates are included."
            {error}
          />
        </div>
        {#if error}<Notice role="alert" tone="warning"
            >The start date must be on or before the end date.</Notice
          >{/if}
        <div class="action-examples">
          <Button type="submit" variant="primary">Apply filters</Button><Button
            type="reset"
            variant="quiet">Clear filters</Button
          ><Button onclick={() => (query = "Jane Smith")}
            >Use example name</Button
          >
        </div>
      </form>
      <div class="filter-status">
        <Notice role="status" tone="accent">{status}</Notice>
      </div>
      <div class="disabled-field">
        <Field
          label="Registration reference"
          value="Available with a declaration"
          disabled
          hint="Disabled fields cannot be edited."
        />
      </div>
      <form class="default-field">
        <Field
          label="Funder name"
          defaultValue="Example Foundation"
          hint="Edit the name, then restore its initial value."
        />
        <div><Button type="reset">Restore name</Button></div>
      </form>
      <details>
        <summary>Using fields</summary>
        <pre><code
            >{'<Field label="Registered to" type="date" bind:value={to}\n  hint="Both dates are included." error={dateError} />'}</code
          ></pre>
      </details>
    </section>
    <section id="feedback" aria-labelledby="feedback-heading">
      <h2 id="feedback-heading">Feedback</h2>
      <p>
        Explain what the data covers and how to recover. Reserve alerts for
        problems that need immediate attention.
      </p>
      <div class="notices">
        <Notice
          >Values reflect stored declarations. Some entries do not contain a
          monetary amount.</Notice
        ><Notice tone="accent"
          >Registration dates include both ends of the selected period.</Notice
        ><Notice tone="warning"
          >No declarations match these dates. Clear or change the dates.</Notice
        >
      </div>
      <details>
        <summary>Using notices</summary>
        <pre><code
            >{'<Notice role="status" tone="accent">Filters applied.</Notice>\n<Notice role="alert" tone="warning">Check the date range.</Notice>'}</code
          ></pre>
      </details>
    </section>
  </main></AppShell
>

<style>
  .reference {
    width: min(100%, 1000px);
    margin-top: 56px;
  }
  .intro {
    max-width: 720px;
    margin-bottom: 56px;
  }
  h1 {
    font-size: clamp(32px, 4vw, 48px);
  }
  p {
    max-width: 72ch;
    color: var(--muted);
    font-size: var(--text-body);
    line-height: 1.8;
  }
  nav {
    display: flex;
    flex-wrap: wrap;
    gap: 12px 24px;
    margin-top: 28px;
    font-size: var(--text-body);
  }
  nav a {
    text-underline-offset: 5px;
  }
  section {
    border-top: 1px solid var(--line);
    padding: 32px 0 40px;
    scroll-margin-top: 24px;
  }
  h2 {
    margin: 0;
    font-size: var(--text-heading);
    font-weight: 600;
    letter-spacing: -0.5px;
  }
  section > p {
    margin: 10px 0 28px;
  }
  .swatches {
    display: grid;
    grid-template-columns: repeat(6, minmax(0, 1fr));
    gap: 16px;
  }
  .swatch {
    display: grid;
    gap: 6px;
    font-size: var(--text-small);
    min-width: 0;
  }
  .color {
    height: 72px;
    border: 1px solid var(--line);
    border-radius: var(--radius-control);
    margin-bottom: 6px;
  }
  .swatch > span:last-child,
  code {
    color: var(--muted);
  }
  code {
    font-size: 12px;
    overflow-wrap: anywhere;
  }
  .foundation-notes {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 32px;
    margin-top: 20px;
  }
  .type-samples {
    display: grid;
    gap: 24px;
  }
  .type-samples > div {
    display: grid;
    gap: 8px;
  }
  .sample-heading {
    font-size: var(--text-heading);
    font-weight: 600;
  }
  .sample-body {
    font-size: var(--text-body);
    line-height: 1.8;
  }
  .sample-context {
    font-size: var(--text-small);
    color: var(--muted);
    line-height: 1.6;
  }
  .action-examples {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
    align-items: center;
  }
  .funder-example {
    --accent: var(--accent-funder);
    border-left: 2px solid var(--accent);
    padding-left: 20px;
    margin-top: 28px;
  }
  .funder-example > span {
    display: block;
    color: var(--accent);
    margin-bottom: 12px;
    font-size: var(--text-body);
  }
  details {
    margin-top: 28px;
    font-size: var(--text-body);
  }
  summary {
    color: var(--muted);
    cursor: pointer;
  }
  summary:focus-visible {
    outline: var(--focus-width) solid var(--accent);
    outline-offset: var(--focus-offset);
  }
  pre {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    background: var(--surface);
    padding: 16px;
    line-height: 1.8;
    border-radius: var(--radius-control);
  }
  form {
    display: grid;
    gap: 20px;
    max-width: 640px;
  }
  .context {
    margin: -10px 0 0;
    font-size: var(--text-small);
  }
  .date-fields {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 20px;
    align-items: start;
  }
  .filter-status {
    margin-top: 24px;
  }
  .disabled-field {
    max-width: 360px;
    margin-top: 28px;
  }
  .default-field {
    margin-top: 28px;
  }
  .notices {
    display: grid;
    gap: 16px;
  }
  @media (max-width: 700px) {
    .swatches {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }
    .foundation-notes {
      grid-template-columns: 1fr;
      gap: 0;
    }
  }
  @media (max-width: 480px) {
    .reference {
      margin-top: 36px;
    }
    .intro {
      margin-bottom: 36px;
    }
    .date-fields {
      grid-template-columns: 1fr;
    }
    .swatches {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
