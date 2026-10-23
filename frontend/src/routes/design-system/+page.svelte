<script lang="ts">
  import { onMount } from "svelte";
  import AppShell from "../../AppShell.svelte";
  import {
    CategoryWheel,
    RankingList,
    Icon,
    Badge,
    IconIndicator,
    SearchField,
    ViewLinks,
    ViewLink,
    Pagination,
    Spinner,
    SkeletonRows,
    MessageState,
    SectionHeading,
    ResultRow,
    HighlightedText,
    DeclarationList,
    DeclarationCard,
    ValueRow,
    IdentityHeader,
    Metric,
    DataTable,
    SignedBarList,
    BarChart,
    PieChart,
    Disclosure,
    Button,
    ButtonLink,
    Field,
    Notice,
  } from "../../lib/ui";
  let exampleCategory = $state("members");
  const wheelOptions = [
    {
      id: "members",
      label: "MPs",
      icon: "parliament",
      accent: "member",
      path: "M138 8 A130 130 0 0 0 8 138 L98 138 A40 40 0 0 1 138 98 Z",
      rimPath: "M138 8 A130 130 0 0 0 8 138",
      left: 0,
      top: 0,
      x: 73,
      y: 73,
    },
    {
      id: "individuals",
      label: "Individuals",
      icon: "person",
      accent: "funder",
      path: "M4 8 A130 130 0 0 1 134 138 L44 138 A40 40 0 0 0 4 98 Z",
      rimPath: "M4 8 A130 130 0 0 1 134 138",
      left: 142,
      top: 0,
      x: 69,
      y: 73,
    },
    {
      id: "companies",
      label: "Companies",
      icon: "building",
      accent: "funder",
      path: "M4 134 A130 130 0 0 0 134 4 L44 4 A40 40 0 0 1 4 44 Z",
      rimPath: "M4 134 A130 130 0 0 0 134 4",
      left: 142,
      top: 142,
      x: 54,
      y: 55,
    },
    {
      id: "parties",
      label: "Parties",
      icon: "group",
      accent: "neutral",
      path: "M8 4 A130 130 0 0 0 138 134 L138 44 A40 40 0 0 1 98 4 Z",
      rimPath: "M8 4 A130 130 0 0 0 138 134",
      left: 0,
      top: 142,
      x: 88,
      y: 55,
    },
  ] satisfies import("../../lib/ui").CategoryWheelOption[];
  let exampleSearch = $state("Example name");
  let examplePage = $state(1);
  const chartSlices = [
    {
      label: "Example party",
      value: "£98,765,432.12345678901234567890123456789",
      detail: "2 entries, 1 amount unavailable",
      share: 1,
      path: "",
      color: "var(--accent)",
    },
  ];
  import type { ChartBar } from "../../lib/ui";
  const chartBars: ChartBar[] = [
    {
      x: 20,
      y: 40,
      width: 180,
      height: 200,
      negative: false,
      title: "Jan 2026: £12,345.67",
      tick: { x: 20, label: "Jan 2026", anchor: "start" },
    },
  ];
  const swatches = [
    { name: "Canvas", token: "--canvas" },
    { name: "Surface", token: "--surface" },
    { name: "Text", token: "--text" },
    { name: "Muted text", token: "--muted" },
    { name: "Actions", token: "--action" },
    { name: "Focus", token: "--focus-color" },
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
        ><a href="#fields">Fields</a><a href="#feedback">Feedback</a><a
          href="#patterns">Patterns</a
        ><a href="#data-patterns">Data</a>
      </nav>
    </header>
    <section id="foundations" aria-labelledby="foundations-heading">
      <h2 id="foundations-heading">Foundations</h2>
      <p>
        Black keeps the focus on declarations. The Exposed wordmark is white.
        Purple identifies MPs; teal identifies funders and the landing page's
        financial interests phrase.
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
          Controls share a 6px radius and a visible neutral focus ring.
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
        retain their native keyboard behavior. Actions and focus stay neutral in
        both member and funder contexts.
      </p>
      <div class="action-examples" aria-label="Member actions">
        <ButtonLink href="/search" variant="primary"
          >Search MPs and funders</ButtonLink
        ><ButtonLink href="/methodology">Read methodology</ButtonLink><Button
          variant="quiet">Quiet action</Button
        ><Button variant="subtle">Subtle action</Button><Button disabled
          >Unavailable action</Button
        >
      </div>
      <div class="funder-example">
        <span>Funder context</span>
        <div class="action-examples" aria-label="Funder actions">
          <ButtonLink href="/search" variant="primary">Find a funder</ButtonLink
          ><Button
            variant="quiet"
            onclick={() => (status = "Funder example selected.")}
            >Show funder context</Button
          ><Button>Secondary funder action</Button><Button variant="subtle"
            >Subtle funder action</Button
          >
        </div>
      </div>
      <Disclosure summary="Using actions" variant="plain" class="code-example">
        <pre><code
            >{'<Button onclick={clearDates} disabled={!from && !to}>Clear dates</Button>\n<ButtonLink href="/search" variant="primary">Search MPs and funders</ButtonLink>'}</code
          ></pre>
      </Disclosure>
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
      <Disclosure summary="Using fields" variant="plain" class="code-example">
        <pre><code
            >{'<Field label="Registered to" type="date" bind:value={to}\n  hint="Both dates are included." error={dateError} />'}</code
          ></pre>
      </Disclosure>
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
      <Disclosure summary="Using notices" variant="plain" class="code-example">
        <pre><code
            >{'<Notice role="status" tone="accent">Filters applied.</Notice>\n<Notice role="alert" tone="warning">Check the date range.</Notice>'}</code
          ></pre>
      </Disclosure>
    </section>

    <section id="patterns" aria-labelledby="patterns-heading">
      <SectionHeading id="patterns-heading" title="Shared patterns"
        ><p>
          Native navigation, search, exact values and data presentation used by
          the application.
        </p></SectionHeading
      >
      <div class="pattern-stack">
        <ViewLinks label="Example views"
          ><ViewLink href="#patterns" aria-current="page">Summary</ViewLink
          ><ViewLink href="#data-patterns">Declaration history</ViewLink
          ></ViewLinks
        >
        <div class="entity-legend">
          <Badge variant="legend"><Icon name="parliament" size={16} />MP</Badge
          ><Badge variant="legend" class="result-palette-example"
            ><Icon name="coin" size={16} />Funder</Badge
          ><IconIndicator icon="parliament" label="Example MP" /><IconIndicator
            icon="coin"
            label="Example funder"
          />
        </div>
        <SearchField
          label="Example live search"
          value={exampleSearch}
          oninput={(event) => (exampleSearch = event.currentTarget.value)}
          onclear={() => (exampleSearch = "")}
          shortcut="Ctrl K"
          >{#snippet help()}Clear or edit this local example. No records are
            requested.{/snippet}</SearchField
        >
        <SearchField
          label="Disabled search"
          value="Cannot clear"
          disabled
          onclear={() => (exampleSearch = "unexpected")}
          >{#snippet help()}Disabled controls cannot change values.{/snippet}</SearchField
        >
        <SearchField
          label="Read only search"
          value="Cannot edit"
          readonly
          onclear={() => (exampleSearch = "unexpected")}
          >{#snippet help()}Read only search preserves its value.{/snippet}</SearchField
        >
        <Pagination
          page={examplePage}
          pageCount={2}
          label="Example pages"
          onprevious={() => (examplePage -= 1)}
          onnext={() => (examplePage += 1)}
        />
        <MessageState title="Start with a name" icon="search"
          ><p>Choose an MP or funder to explore.</p></MessageState
        >
        <MessageState title="No matches" icon="search"
          ><p>Try a different spelling.</p></MessageState
        >
        <MessageState title="Could not load" icon="retry"
          ><p>This is an example error.</p>
          {#snippet actions()}<Button
              onclick={() => (status = "Example retry requested.")}
              >Retry example</Button
            >{/snippet}</MessageState
        >
        <MessageState title="Loading example"
          ><Spinner /><SkeletonRows widths={[60, 44]} /></MessageState
        >
        <ul class="results-list">
          <ResultRow class="result-palette-example"
            ><IconIndicator icon="coin" label="Example result" />
            <div class="entity-text">
              <a class="entity-name" href="#data-patterns"
                ><HighlightedText
                  parts={[
                    { text: "Example ", matched: true },
                    { text: "Foundation <literal text>", matched: false },
                  ]}
                /></a
              ><span class="entity-alias">Matched alias: Example Trust</span>
            </div>
            <Badge>Organisation</Badge></ResultRow
          >
        </ul>
      </div>
    </section>
    <section id="ranking-patterns" aria-labelledby="ranking-patterns-heading">
      <SectionHeading
        id="ranking-patterns-heading"
        title="Category selection and rankings"
      />
      <CategoryWheel
        options={wheelOptions}
        selected={exampleCategory}
        onselect={(id) => (exampleCategory = id)}
        label="Example ranking category"
      />
      <RankingList
        label="Example rankings"
        rows={[
          {
            key: exampleCategory + "1",
            name: "Jane Example",
            context: "Example Party · Example constituency",
            href: "/members/1",
            amount: "£90,071,992,547,409,993.123456789",
            incomplete: true,
            width: 100,
            negative: false,
            accent: exampleCategory === "members" ? "member" : "funder",
          },
          {
            key: exampleCategory + "2",
            name: "Example party",
            context: "2 declarations",
            amount: "£0.00",
            incomplete: false,
            width: 0,
            negative: false,
            accent: "neutral",
          },
        ]}
      />
    </section>

    <section id="data-patterns" aria-labelledby="data-patterns-heading">
      <SectionHeading id="data-patterns-heading" title="Identity and data" />
      <IdentityHeader name="Example member with a long recorded name" level={3}
        >{#snippet kind()}<Badge variant="profile"
            ><Icon name="parliament" />MP</Badge
          >{/snippet}{#snippet metadata()}Example party<span>Current MP</span
          >{/snippet}{#snippet values()}<Metric
            level={3}
            label="Known declared value"
            value="£98,765,432.12345678901234567890123456789"
          /><Metric
            level={3}
            label="Unavailable value"
            value="Amount unavailable"
            available={false}
          />{/snippet}</IdentityHeader
      >
      <Metric
        variant="expanded"
        align="end"
        level={3}
        label="Declared amounts"
        value="£42,500.75"
      />
      <DeclarationList
        ><DeclarationCard
          >{#snippet heading()}<h3>Employment</h3>
            <a class="declaration-reference" href="#data-patterns"
              >Declaration 123</a
            >{/snippet}{#snippet registered()}<time datetime="2026-01-01"
              >1 January 2026</time
            >{/snippet}
          <ul class="funding-list">
            <ValueRow
              >{#snippet source()}<span>Example Foundation</span><span
                  class="payment-type">Donation</span
                >{/snippet}{#snippet value()}<span class="amount"
                  >£12,345.6789</span
                >{/snippet}</ValueRow
            >
          </ul></DeclarationCard
        ></DeclarationList
      >
      <DataTable caption="Example recorded values"
        >{#snippet head()}<tr
            ><th scope="col">Source</th><th scope="col">Known value</th></tr
          >{/snippet}<tr
          ><th scope="row"><a href="#patterns">Example Foundation</a></th><td
            >£12,345.6789</td
          ></tr
        ></DataTable
      >
      <SignedBarList
        baseline={25}
        bars={[
          {
            label: "Employment",
            value: "£12,345.67",
            width: 75,
            negative: false,
          },
          {
            label: "Adjustment",
            value: "-£1,000",
            width: 25,
            negative: true,
            note: "Some amounts unavailable",
          },
        ]}
      />
      {#each ["member", "funder"] as tone}<div
          class:funder-page={tone === "funder"}
          class="chart-example"
        >
          <BarChart
            title={`${tone} monthly example`}
            description="Precalculated geometry with exact formatted values."
            bars={chartBars}
            baseline={240}
            minimum="£0"
            minimumY={265}
          />
          <div class="funder-party-chart">
            <PieChart label={`${tone} party example`} slices={chartSlices} />
          </div>
        </div>{/each}
      <Disclosure summary="Recorded names example" open
        ><ul>
          <li>Example Foundation</li>
          <li>Example Trust</li>
        </ul></Disclosure
      >
    </section>

    <section aria-labelledby="ownership-heading">
      <SectionHeading
        id="ownership-heading"
        title="Portable compositions"
        level={3}
      />
      <Badge class="custom-badge">Additive result badge</Badge>
      <Badge variant="profile" class="custom-profile">Profile badge</Badge>
      <Badge variant="legend" class="custom-legend">Legend badge</Badge>
      <MessageState
        title="Centered semantic heading"
        level={2}
        class="custom-state"
        ><p>Centered appearance at level two.</p></MessageState
      >
      <MessageState title="Detail semantic heading" level={2} variant="detail"
        ><p>Detail appearance at level two.</p></MessageState
      >
      <Disclosure
        summary="Plain additive disclosure"
        variant="plain"
        class="custom-disclosure"><p>Native disclosure content.</p></Disclosure
      >
      <Button class={["custom-action", { "active-example": true }]}
        >Array class action</Button
      >
      <ButtonLink href="#ownership-heading" class={{ "custom-link": true }}
        >Object class link</ButtonLink
      >
      <Field label="Class array field" class={["custom-field"]} />
      <Notice class={{ "custom-notice": true }}>Object class notice</Notice>
      <DataTable caption="Three text columns">
        {#snippet head()}<tr
            ><th scope="col">First</th><th scope="col">Second</th><th
              scope="col">Third</th
            ></tr
          >{/snippet}
        <tr><th scope="row">Alpha</th><td>Beta</td><td>Gamma</td></tr>
      </DataTable>
      <ul aria-label="Standalone values">
        <ValueRow>
          {#snippet source()}Standalone source
            <ul><li>Nested native item</li></ul>{/snippet}
          {#snippet value()}<span class="amount">£123.45</span>{/snippet}
        </ValueRow>
      </ul>
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
  :global(.result-palette-example) {
    --accent: var(--accent-funder);
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
  :global(.code-example) {
    margin-top: 28px;
    font-size: var(--text-body);
  }
  :global(.code-example summary) {
    color: var(--muted);
    cursor: pointer;
  }
  :global(.code-example summary:focus-visible) {
    outline: var(--focus-width) solid var(--focus-color);
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

  .pattern-stack {
    display: grid;
    gap: 24px;
  }
  .chart-example {
    min-width: 0;
    margin-block: 24px;
  }
  .chart-example :global(.earnings-chart) {
    max-width: 560px;
  }
  .chart-example :global(.funder-party-chart) {
    max-width: 560px;
  }
</style>
