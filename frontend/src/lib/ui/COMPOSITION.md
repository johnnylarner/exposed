# Composition recipes

Each example is a complete Svelte component placed at `src/Example.svelte`. Load `../lib/ui/styles.css` once in `src/routes/+layout.svelte`. These examples use static presentation data; production features supply real URLs and formatted values.

Actions and focus remain neutral in every entity context. Override `--accent` only for entity meaning, such as funder badges, highlights and charts.

## Search result

`ResultRow` renders `li`. The caller owns the anchor, metadata and list container. The UI stylesheet owns `.entity-text`, `.entity-name` and `.entity-alias`.

```svelte
<script lang="ts">
  import { ResultRow, Badge, IconIndicator } from "./lib/ui";
</script>

<ul class="results-list">
  <ResultRow label="Example Foundation, funder">
    <IconIndicator icon="building" label="Organisation" />
    <div class="entity-text">
      <a
        class="entity-name"
        href="/funders/11111111-1111-4111-8111-111111111111"
        >Example Foundation</a
      >
      <span class="entity-alias">Also recorded as Example Trust</span>
    </div>
    <Badge>Organisation</Badge>
  </ResultRow>
</ul>
```

## Declaration and value

`DeclarationList` renders `ol`, `DeclarationCard` renders `li`, and `ValueRow` renders a nested `li`. `.funding-list` is the UI list recipe for reset and spacing. The registered snippet is already wrapped in `p`, so supply inline content. The UI stylesheet owns `.payment-type` and `.amount`; the caller supplies the formatted amount.

```svelte
<script lang="ts">
  import { DeclarationList, DeclarationCard, ValueRow } from "./lib/ui";
</script>

<DeclarationList label="Example declarations">
  <DeclarationCard>
    {#snippet heading()}<h3>Employment and earnings</h3>{/snippet}
    {#snippet registered()}<time datetime="2026-01-20">20 January 2026</time
      >{/snippet}
    <ul class="funding-list">
      <ValueRow>
        {#snippet source()}
          <a href="/funders/11111111-1111-4111-8111-111111111111"
            >Example Foundation</a
          >
          <span class="payment-type">Consultancy</span>
        {/snippet}
        {#snippet value()}<span class="amount">£1,234.56</span>{/snippet}
      </ValueRow>
    </ul>
  </DeclarationCard>
</DeclarationList>
```

## Identity and metric

`IdentityHeader` owns its title, metadata paragraph and grid. Supply inline metadata and a native navigation composition. View-specific controls and metrics belong after identity navigation, inside the active view. The optional `values` snippet remains available for standalone identities without view navigation. `Metric` takes exact formatted strings without interpreting money.

```svelte
<script lang="ts">
  import {
    IdentityHeader,
    Badge,
    Icon,
    Metric,
    ViewLinks,
    ViewLink,
  } from "./lib/ui";
</script>

<IdentityHeader name="Jane Example">
  {#snippet kind()}<Badge variant="profile"><Icon name="parliament" />MP</Badge
    >{/snippet}
  {#snippet metadata()}Example Party<span>Example constituency</span>{/snippet}
  {#snippet navigation()}
    <ViewLinks label="Example views">
      <ViewLink href="/members/1" aria-current="page">Summary</ViewLink>
      <ViewLink href="/members/1/history">Declaration history</ViewLink>
    </ViewLinks>
  {/snippet}
</IdentityHeader>

<section aria-label="Summary">
  <Metric variant="expanded" label="Declared amounts" value="£1,234.56" />
</section>
```

## Native table columns

`DataTable` supplies the caption, `thead`, `tbody` and neutral cell rules. Use actual `th`/`td` classes in the caller for widths and numeric alignment. [FunderSummary](../../FunderSummary.svelte) is the production example. It owns `.recipient-name` and `.recipient-total`; those classes are not reusable UI APIs.

## Period or other native-link filters

Use `<ViewLinks variant="filter" label="Reporting period">` with native `ViewLink` children. Set `aria-current="page"` on the selected destination. Features own URL state and labels; the wrapping presentation stays neutral in every entity context.

For right-aligned filters, set `align="end"` on ViewLinks. Keep the navigation at full available width so wrapped lines end at the same edge. Put committed results in `<ContentChange changeKey={period}>` after the controls. Keep the existing content and key while navigation is pending; update both together when server data arrives. Do not key or duplicate the child subtree.
