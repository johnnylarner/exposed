<script lang="ts">
  import Icon from "./lib/Icon.svelte";
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
  let input: HTMLInputElement;
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
    input.focus();
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
      input.focus();
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
      input.focus();
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
    <label class="visually-hidden" for="entity-search"
      >Search MPs and funders</label
    >
    <div class="search-field">
      <span class="search-icon"><Icon name="search" size={24} /></span>
      <input
        id="entity-search"
        type="search"
        bind:this={input}
        value={query}
        oninput={(event) => updateQuery(event.currentTarget.value)}
        oncompositionstart={() => (composing = true)}
        oncompositionend={() => (composing = false)}
        placeholder="Enter an MP or funder’s name"
        autocomplete="off"
        spellcheck="false"
        aria-describedby="search-help"
      />
      {#if query}
        <button
          class="clear-button"
          type="button"
          aria-label="Clear search"
          onclick={clearSearch}><Icon name="close" size={18} /></button
        >
      {:else}
        <kbd class="search-shortcut" aria-hidden="true"
          >{searchFocusShortcut.hint}</kbd
        >
      {/if}
    </div>
    <p class="search-help" id="search-help">
      {#if term && charactersNeeded}
        Type {charactersNeeded} more {charactersNeeded === 1
          ? "character"
          : "characters"} to search.
      {:else if !term}
        Search by name. Results appear as you type.
      {:else}
        MPs and funders, ordered by the closest name match.
      {/if}
    </p>
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
      <div class="starting-state">
        <div class="start-symbol" aria-hidden="true">
          <Icon name="search" size={27} />
        </div>
        <h2>Start with a name</h2>
        <p>Look up an MP, a company, or an individual funder.</p>
        <div class="examples">
          <span>Try searching</span>{#each ["John", "HSBC"] as example}<button
              type="button"
              onclick={() => {
                updateQuery(example);
                input.focus();
              }}>{example}</button
            >{/each}
        </div>
      </div>
    {:else if searchState.status === "loading"}
      <div class="results-heading">
        <h2>Searching</h2>
        <span class="loading-indicator" aria-hidden="true"></span>
      </div>
      <div class="loading-rows" aria-hidden="true">
        {#each [60, 44, 72, 53] as width}<div class="loading-row">
            <span class="skeleton-icon"></span><span
              class="skeleton-name"
              style:width={`${width}%`}
            ></span>
          </div>{/each}
      </div>
    {:else if searchState.status === "error"}
      <div class="message-state" role="alert">
        <span class="state-icon"><Icon name="retry" size={24} /></span>
        <h2>Search couldn’t load</h2>
        <p>{searchState.message}</p>
        <button
          class="retry-button"
          type="button"
          onclick={retrySearch}
          ><Icon name="retry" size={16} />Try again</button
        >
      </div>
    {:else if searchState.entities.length === 0}
      <div class="message-state">
        <span class="state-icon"><Icon name="search" size={24} /></span>
        <h2>No matches for “{searchState.term}”</h2>
        <p>Try a surname, a shorter company name, or a different spelling.</p>
        <button
          class="text-button"
          type="button"
          onclick={() => {
            input.focus();
            input.select();
          }}>Edit your search</button
        >
      </div>
    {:else}
      <div class="results-heading">
        <h2>
          {searchState.entities.length}
          {searchState.entities.length === 1 ? "result" : "results"} shown
        </h2>
        <span>Best matches first</span>
      </div>
      <div class="entity-legend" aria-label="Entity icon key">
        <span><Icon name="parliament" size={16} />MP</span>
        <span><Icon name="coin" size={16} />Funder</span>
      </div>
      <ul class="results-list">
        {#each searchState.entities as entity, index}
          <li class="entity-row">
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
                >{#each highlightName(entity.name, searchState.term) as part}{#if part.matched}<mark
                      >{part.text}</mark
                    >{:else}{part.text}{/if}{/each}</a
              >
              {#if entity.matchSource.kind === "alias"}
                <span class="entity-alias" id={`matched-alias-${entity.id}`}
                  >Matched alias: {#each highlightName(entity.matchSource.name, searchState.term) as part}{#if part.matched}<mark
                        >{part.text}</mark
                      >{:else}{part.text}{/if}{/each}</span
                >
              {/if}
            </div>
            {#if entity.kind === "Funder"}
              <EntityIndicator {entity} detail="kind" />
            {/if}
          </li>
        {/each}
      </ul>
      {#if atLimit(searchState.entities)}<p class="results-note">
          Showing the closest matches. Add more of the name to narrow your
          search.
        </p>{/if}
    {/if}
  </section>
</main>
