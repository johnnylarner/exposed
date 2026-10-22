# Exposed UI

The live reference at `/design-system` runs without the declarations API.
Import `Button`, `ButtonLink`, `Field` and `Notice` from `$lib/ui`. The root layout loads `tokens.css`. Actions share `actions.css`.

```svelte
<Button onclick={clearDates} disabled={!from && !to}>Clear dates</Button>
<ButtonLink href="/search" variant="primary">Search MPs and funders</ButtonLink>
<Field
  label="Registered from"
  type="date"
  bind:value={from}
  hint="Both dates are included."
/>
<Notice role="alert" tone="warning">Check the date range.</Notice>
```

Buttons default to `type="button"`. Set submit or reset for form actions. Links require href. Both forward native attributes and handlers and accept class, variant (primary, secondary, quiet) and size (regular, large).

Field supports bindable string values for text, search and date inputs. A visible label is required. Generated IDs remain stable through hydration. Pass an explicit ID for external references. Hint and error IDs merge with caller aria-describedby. Errors set aria-invalid. Native attributes go to the input; class goes to the wrapper.

Set `defaultValue` to a string for an initial value that native form reset restores. A supplied `value` takes precedence on the initial render.

Notice forwards native div attributes. Callers choose the role for announcements. Neutral gives context, accent gives updates, and warning gives recovery instructions. Tone alone does not announce content.

Tokens own foundational colors, type, spacing, radius and focus. Set --accent to var(--accent-funder) on a containing view for teal controls. Keep domain calculations, filtering, URLs, analytics and asynchronous effects in callers. Pages own layout. Chart values remain with charts. Add components when repeated behavior needs one owner and remove obsolete caller styles during migration.
