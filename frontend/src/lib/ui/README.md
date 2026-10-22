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

Subtle compact actions are used for suggested searches. Inline size preserves text action placement for back and history links.

Buttons default to `type="button"`. Set submit or reset for form actions. Links require href. Both forward native attributes and handlers and accept class, variant (primary, secondary, quiet, subtle) and size (regular, large, compact, inline).

Field supports bindable string values for text, search and date inputs. A visible label is required. Generated IDs remain stable through hydration. Pass an explicit ID for external references. Hint and error IDs merge with caller aria-describedby. Errors set aria-invalid. Native attributes go to the input; class goes to the wrapper.

Set `defaultValue` to a string for an initial value that native form reset restores. A supplied `value` takes precedence on the initial render.

Notice forwards native div attributes. Callers choose the role for announcements. Neutral gives context, accent gives updates, and warning gives recovery instructions. Tone alone does not announce content.

Tokens own foundational colors, type, spacing, radius and focus. Set --accent to var(--accent-funder) on a containing view for teal controls. Keep domain calculations, filtering, URLs, analytics and asynchronous effects in callers. Pages own layout. Formatted chart values and geometry come from feature adapters. Add components when repeated behavior needs one owner and remove obsolete caller styles during migration.

## Pattern inventory

Every component below has a live specimen at `/design-system`.

| Pattern                                        | Component             | Production caller                                   |
| ---------------------------------------------- | --------------------- | --------------------------------------------------- |
| Native actions                                 | Button, ButtonLink    | SearchPage, EntityDetails, histories, MemberSummary |
| Labeled native fields                          | Field                 | MemberHistory, FunderHistory                        |
| Search input, clear action, help and shortcut  | SearchField           | SearchPage                                          |
| Icons and finite icon vocabulary               | Icon, IconName        | SearchPage, EntityDetails, identities               |
| Inline type and role labels                    | Badge                 | EntityIndicator, MemberIdentity, FunderIdentity     |
| Focus, hover and Escape icon hints             | IconIndicator         | EntityIndicator                                     |
| Active native anchor navigation                | ViewLinks, ViewLink   | MemberIdentity, FunderIdentity                      |
| Bounded previous and next actions              | Pagination            | MemberHistory, FunderHistory                        |
| Decorative progress                            | Spinner, SkeletonRows | SearchPage, EntityDetails                           |
| Start, empty, error and loading content        | MessageState          | SearchPage, EntityDetails                           |
| Heading and context                            | SectionHeading        | Histories, MemberSummary, FunderSummary             |
| Responsive search rows                         | ResultRow             | SearchPage                                          |
| Literal highlighted text segments              | HighlightedText       | SearchPage names and matched aliases                |
| Ordered declarations                           | DeclarationList       | MemberDeclarations, FunderDeclarations              |
| Declaration heading, date and content          | DeclarationCard       | MemberDeclarations, FunderDeclarations              |
| Funding source and exact value columns         | ValueRow              | MemberDeclarations, FunderDeclarations              |
| Identity title, metadata, values and views     | IdentityHeader        | MemberIdentity, FunderIdentity                      |
| Exact scrollable totals and unavailable values | Metric                | MemberIdentity, FunderIdentity                      |
| Native caption, column headings and body       | DataTable             | FunderSummary                                       |
| Signed horizontal bars                         | SignedBarList         | MemberSummary                                       |
| Monthly SVG bars, titles and ticks             | BarChart              | RegistrationTimeline                                |
| Pie slices, legend and persistent tooltip      | PieChart              | FunderSummary                                       |
| Native disclosure                              | Disclosure            | FunderSummary recorded names                        |
| Coverage and date-range messages               | Notice                | FunderSummary, MemberHistory, FunderHistory         |

## Content and interaction contracts

Import components and their public types from `$lib/ui`. `patterns.css` owns shared presentation and is loaded once by the root layout. `actions.css` owns Button and ButtonLink styles. Field and Notice retain their scoped styles. Do not copy these selectors into feature files.

SearchField accepts a bindable string `value`, a string `defaultValue`, a bindable native `input` reference, a label, a help snippet, an optional shortcut and an `onclear` handler. Native input attributes and events reach the input. The generated help ID merges with caller `aria-describedby`. Disabled and read only fields also disable their clear button. The feature owns query updates, IME state, debounce, keyboard shortcuts, navigation and analytics. SearchPage keeps its focus behavior through the input reference.

MessageState takes a title, heading level 1 or 2, optional icon, content and action snippets. The caller explicitly selects an announcement role. Spinner and SkeletonRows are decorative. SectionHeading requires a stable caller ID and supports heading levels 1 through 3. ViewLink forwards native anchor attributes, including `aria-current`; callers own destinations and analytics. Pagination owns button availability while the caller owns the current page and filtered data.

IdentityHeader accepts `name`, `kind` and `metadata` snippets with optional `values` and `navigation`. DeclarationCard accepts `heading`, `registered` and content snippets. ValueRow accepts `source` and `value` snippets. DeclarationList and ValueRow preserve native ordered-list and nested unordered-list semantics. DataTable requires a plain-text caption, a `head` snippet containing native header rows, and body rows as children. Callers supply actual `th`, `td`, links, `scope` attributes and exact text.

Metric takes an already formatted value and an explicit availability flag. Available values are labeled regions that keyboard users can scroll without truncating digits. `regionLabel` can differ from the visible heading. Unavailable text stays ordinary text. Callers decide availability, wording and precision.

Charts accept presentation models only. `SignedBar` contains a label, exact formatted value, numeric width, sign and optional note. `ChartBar` contains SVG geometry, a formatted exact title and optional positioned tick. `PieSlice` contains a computed share, SVG path, color, label, exact formatted value and detail text. Financial parsing, aggregation, sign classification, share calculations, month labeling and coverage messages stay in feature adapters. UI components never parse amounts or derive financial availability from strings.

BarChart and IconIndicator generate per-instance association IDs. PieChart generates its tooltip ID and owns pointer persistence, delayed dismissal, Escape suppression until pointer movement, touch exclusion, scroll and resize dismissal, and timer cleanup. The gallery renders repeated instances to exercise these associations. HighlightedText renders text nodes and mark elements, never HTML.

Disclosure forwards native details attributes and supports bindable `open`. The summary remains a native keyboard control.

## Remaining composition

AppShell owns the application header, footer, skip link and native brand and content anchors. Landing hero, methodology article, search introduction, detail page sections and summary panel arrangement remain page layouts. Root `styles.css` owns global typography, page layout and documented shared text treatments such as `.amount`, `.entry-count`, `.partial-note`, `.filter-context` and `.empty-details`. Those classes carry formatting and context without requiring a component for each span. Reusable controls, rows, charts, navigation, identity and declaration presentation belong to `lib/ui`.
