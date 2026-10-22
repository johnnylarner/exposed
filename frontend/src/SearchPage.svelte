<script lang="ts">
  import {
    Icon,
    SearchField,
    Spinner,
    SkeletonRows,
    ResultRow,
    HighlightedText,
    MessageState,
    Button,
    Badge,
  } from "./lib/ui";
  import EntityIndicator from "./lib/EntityIndicator.svelte";
  import {
    entityHref,
    highlightName,
    searchConfiguration,
    type Entity,
  } from "./lib/search";

  import { onMount, untrack } from "svelte";
  import { page } from "$app/state";
  import { goto, afterNavigate } from "$app/navigation";
  import posthog from "posthog-js";
  import { posthogLogger } from "./lib/posthog-logger";
  import type { PageModel, SearchResult } from "./lib/page";
  let { model }: { model: Extract<PageModel, { kind: "search" }> } = $props();
  type SearchState = SearchResult | { status: "loading"; term: string };
  type SearchFocusShortcut =
    { modifier: "meta"; hint: "⌘ K" } | { modifier: "ctrl"; hint: "Ctrl K" };
  let searchFocusShortcut = $state<SearchFocusShortcut>({
    modifier: "ctrl",
    hint: "Ctrl K",
  });
  onMount(() => {
    if (/Mac/i.test(navigator.platform))
      searchFocusShortcut = { modifier: "meta", hint: "⌘ K" };
  });
  let query = $state(untrack(() => model.query));
  afterNavigate(({ type }) => {
    if (type === "popstate" || type === "link") {
      query = page.url.searchParams.get("q") ?? "";
    }
  });
  let composing = $state(false);
  let attempt = $state(0);
  let input = $state<HTMLInputElement>();
  let pending = $state(false);
  const term = $derived(query.trim());
  const charactersNeeded = $derived(
    Math.max(0, searchConfiguration.minimumCharacters - [...term].length),
  );
  const searchState = $derived<SearchState>(
    composing || charactersNeeded
      ? { status: "idle" }
      : pending || model.query.trim() !== term
        ? { status: "loading", term }
        : model.result,
  );
  const announcement = $derived(
    searchState.status === "loading"
      ? "Searching…"
      : searchState.status === "success"
        ? `${searchState.entities.length} ${searchState.entities.length === 1 ? "result" : "results"} shown for ${searchState.term}.`
        : "",
  );

  $effect(() => {
    const currentTerm = term;
    const currentAttempt = attempt;
    if (
      composing ||
      [...currentTerm].length < searchConfiguration.minimumCharacters
    )
      return;
    if (!currentAttempt && untrack(() => model.query.trim() === currentTerm))
      return;
    let current = true;
    const timer = setTimeout(
      async () => {
        const url = new URL(window.location.href);
        url.searchParams.set("q", currentTerm);
        const queryLength = [...currentTerm].length;
        posthog.capture("search_submitted", { query_length: queryLength });
        posthogLogger.info("search submitted", { query_length: queryLength });
        pending = true;
        try {
          await goto(url, { replace: true, reset: false, refreshAll: true });
        } finally {
          if (current) pending = false;
        }
      },
      currentAttempt ? 0 : searchConfiguration.debounceMs,
    );
    return () => {
      current = false;
      pending = false;
      clearTimeout(timer);
    };
  });

  function updateQuery(value: string) {
    attempt = 0;
    query = value;
    const url = new URL(window.location.href);
    if (value.trim()) url.searchParams.set("q", value.trim());
    else url.searchParams.delete("q");
    void goto(url, {
      shallow: true,
      replace: true,
      reset: false,
      state: page.state,
    });
  }

  function clearSearch() {
    updateQuery("");
    input?.focus();
  }

  function retrySearch() {
    posthogLogger.warn("search retry requested", {
      query_length: [...term].length,
    });
    attempt += 1;
  }

  function shortcut(event: KeyboardEvent) {
    if (
      event.key.toLowerCase() === "k" &&
      !event.defaultPrevented &&
      !event.isComposing &&
      !composing &&
      !event.shiftKey &&
      !event.altKey &&
      (searchFocusShortcut.modifier === "meta"
        ? event.metaKey && !event.ctrlKey
        : event.ctrlKey && !event.metaKey)
    ) {
      event.preventDefault();
      input?.focus();
      return;
    }
    const target = event.target;
    const editing =
      target instanceof HTMLElement &&
      (target.isContentEditable ||
        ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName));
    if (
      event.key === "/" &&
      !editing &&
      !event.ctrlKey &&
      !event.metaKey &&
      !event.altKey
    ) {
      event.preventDefault();
      input?.focus();
    }
    if (event.key === "Escape" && target === input && !event.isComposing)
      clearSearch();
  }

  function atLimit(entities: Entity[]) {
    return ["MP", "Funder"].some(
      (kind) =>
        entities.filter((entity) => entity.kind === kind).length >=
        searchConfiguration.maxEntriesPerType,
    );
  }
</script>

<svelte:window onkeydown={shortcut} />

<main id="search">
  <div class="search-intro">
    <h1>Search MPs and funders</h1>
    <p>Find a name in the Register of Members’ Financial Interests.</p>
  </div>

  <form
    role="search"
    onsubmit={(event) => {
      event.preventDefault();
      if (!composing && !charactersNeeded) {
        attempt += 1;
      }
    }}
  >
    <SearchField
      id="entity-search"
      label="Search MPs and funders"
      bind:input
      value={query}
      shortcut={searchFocusShortcut.hint}
      onclear={clearSearch}
      oninput={(event) => updateQuery(event.currentTarget.value)}
      oncompositionstart={() => (composing = true)}
      oncompositionend={() => (composing = false)}
      placeholder="Enter an MP or funder’s name"
      autocomplete="off"
      spellcheck="false"
      >{#snippet help()}
        {#if term && charactersNeeded}
          Type {charactersNeeded} more {charactersNeeded === 1
            ? "character"
            : "characters"} to search.
        {:else if !term}
          Search by name. Results appear as you type.
        {:else}
          MPs and funders, ordered by the closest name match.
        {/if}
      {/snippet}</SearchField
    >
  </form>

  <p
    class="visually-hidden"
    role="status"
    aria-live="polite"
    aria-atomic="true"
  >
    {announcement}
  </p>
  <section
    class="results-region"
    aria-label="Search results"
    aria-busy={searchState.status === "loading"}
  >
    {#if searchState.status === "idle"}
      <MessageState title="Start with a name" icon="search">
        <p>Look up an MP, a company, or an individual funder.</p>
        <div class="examples">
          <span>Try searching</span>{#each ["John", "HSBC"] as example}<Button
              variant="subtle"
              size="compact"
              onclick={() => {
                updateQuery(example);
                input?.focus();
              }}>{example}</Button
            >{/each}
        </div>
      </MessageState>
    {:else if searchState.status === "loading"}
      <div class="results-heading">
        <h2>Searching</h2>
        <Spinner />
      </div>
      <SkeletonRows />
    {:else if searchState.status === "error"}
      <MessageState title="Search couldn’t load" icon="retry" role="alert">
        <p>{searchState.message}</p>
        <Button onclick={retrySearch}
          ><Icon name="retry" size={16} />Try again</Button
        >
      </MessageState>
    {:else if searchState.entities.length === 0}
      <MessageState
        title={`No matches for “${searchState.term}”`}
        icon="search"
      >
        <p>Try a surname, a shorter company name, or a different spelling.</p>
        <Button
          variant="quiet"
          onclick={() => {
            input?.focus();
            input?.select();
          }}>Edit your search</Button
        >
      </MessageState>
    {:else}
      <div class="results-heading">
        <h2>
          {searchState.entities.length}
          {searchState.entities.length === 1 ? "result" : "results"} shown
        </h2>
        <span>Best matches first</span>
      </div>
      <div class="entity-legend" aria-label="Entity icon key">
        <Badge variant="legend"><Icon name="parliament" size={16} />MP</Badge>
        <Badge variant="legend" class="funder-accent"
          ><Icon name="coin" size={16} />Funder</Badge
        >
      </div>
      <ul class="results-list">
        {#each searchState.entities as entity, index}
          <ResultRow class={entity.kind === "Funder" ? "funder-accent" : ""}>
            <EntityIndicator {entity} />
            <div class="entity-text">
              <a
                href={entityHref(entity, page.url.searchParams)}
                class="entity-name"
                aria-describedby={entity.matchSource.kind === "alias"
                  ? `matched-alias-${entity.id}`
                  : undefined}
                onclick={() => {
                  const selection = {
                    entity_type: entity.kind,
                    match_source: entity.matchSource.kind,
                    result_position: index + 1,
                  };
                  posthog.capture("search_result_selected", selection);
                  posthogLogger.info("search result selected", selection);
                }}
                ><HighlightedText
                  parts={highlightName(entity.name, searchState.term)}
                /></a
              >
              {#if entity.matchSource.kind === "alias"}
                <span class="entity-alias" id={`matched-alias-${entity.id}`}
                  >Matched alias: <HighlightedText
                    parts={highlightName(
                      entity.matchSource.name,
                      searchState.term,
                    )}
                  /></span
                >
              {/if}
            </div>
            {#if entity.kind === "Funder"}
              <EntityIndicator {entity} detail="kind" />
            {/if}
          </ResultRow>
        {/each}
      </ul>
      {#if atLimit(searchState.entities)}<p class="results-note">
          Showing the closest matches. Add more of the name to narrow your
          search.
        </p>{/if}
    {/if}
  </section>
</main>

<style>
  :global(.funder-accent) {
    --accent: var(--accent-funder);
  }
</style>
