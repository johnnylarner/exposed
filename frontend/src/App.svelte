<script lang="ts">
  import Icon from "./lib/Icon.svelte";
  import EntityIndicator from "./lib/EntityIndicator.svelte";
  import {
    highlightName,
    readStrictness,
    searchConfiguration,
    searchEntities,
    type Entity,
  } from "./lib/search";

  type SearchState =
    | { status: "idle" }
    | { status: "loading"; term: string }
    | { status: "success"; term: string; entities: Entity[] }
    | { status: "error"; term: string; message: string };

  let query = $state(new URL(window.location.href).searchParams.get("q") ?? "");
  let strictness = $state(
    readStrictness(
      new URL(window.location.href).searchParams.get("strictness"),
    ),
  );
  let composing = $state(false);
  let attempt = $state(0);
  let input: HTMLInputElement;
  let searchState = $state<SearchState>({ status: "idle" });
  const term = $derived(query.trim());
  const charactersNeeded = $derived(
    Math.max(0, searchConfiguration.minimumCharacters - [...term].length),
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
    const currentStrictness = strictness;
    const currentAttempt = attempt;
    if (
      composing ||
      [...currentTerm].length < searchConfiguration.minimumCharacters
    ) {
      searchState = { status: "idle" };
      return;
    }

    const controller = new AbortController();
    let current = true;
    searchState = { status: "loading", term: currentTerm };
    const timer = setTimeout(
      async () => {
        try {
          const entities = await searchEntities(
            currentTerm,
            controller.signal,
            currentStrictness,
          );
          if (current)
            searchState = { status: "success", term: currentTerm, entities };
        } catch (error) {
          if (current)
            searchState = {
              status: "error",
              term: currentTerm,
              message:
                error instanceof Error
                  ? error.message
                  : "Search couldn’t load. Try again.",
            };
        }
      },
      currentAttempt > 0 ? 0 : searchConfiguration.debounceMs,
    );

    return () => {
      current = false;
      clearTimeout(timer);
      controller.abort();
    };
  });

  function updateQuery(value: string) {
    attempt = 0;
    query = value;
    const url = new URL(window.location.href);
    if (value.trim()) url.searchParams.set("q", value.trim());
    else url.searchParams.delete("q");
    window.history.replaceState(null, "", url);
  }

  function updateStrictness(value: string) {
    attempt = 0;
    strictness = readStrictness(value);
    const url = new URL(window.location.href);
    url.searchParams.set("strictness", String(strictness));
    window.history.replaceState(null, "", url);
  }

  function clearSearch() {
    updateQuery("");
    input.focus();
  }

  function shortcut(event: KeyboardEvent) {
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

<svelte:window
  onkeydown={shortcut}
  onpopstate={() => {
    const params = new URL(window.location.href).searchParams;
    query = params.get("q") ?? "";
    strictness = readStrictness(params.get("strictness"));
    attempt = 0;
  }}
/>

<a class="skip-link" href="#search">Skip to search</a>
<div class="app-shell">
  <header class="app-header">
    <a class="wordmark" href="/" aria-label="Exposed home">exposed</a>
    <span class="header-context">UK Parliament</span>
  </header>

  <main id="search">
    <div class="search-intro">
      <h1>Search MPs and funders</h1>
      <p>Find a name in the Register of Members’ Financial Interests.</p>
    </div>

    <form
      role="search"
      onsubmit={(event) => {
        event.preventDefault();
        if (!composing && !charactersNeeded) attempt += 1;
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
          <kbd class="search-shortcut" aria-hidden="true">/</kbd>
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
      <div class="threshold-control">
        <div class="threshold-label">
          <label for="similarity-threshold">Similarity threshold</label>
          <output for="similarity-threshold">{strictness.toFixed(2)}</output>
        </div>
        <input
          id="similarity-threshold"
          type="range"
          min="0"
          max="1"
          step="0.01"
          value={strictness}
          oninput={(event) => updateStrictness(event.currentTarget.value)}
          aria-describedby="threshold-help"
        />
        <div class="threshold-scale" aria-hidden="true">
          <span>Broader matches</span><span>Closer matches</span>
        </div>
        <p id="threshold-help">
          Lower the threshold to include less similar names.
        </p>
      </div>
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
            onclick={() => (attempt += 1)}
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
          {#each searchState.entities as entity}
            <li class="entity-row">
              <EntityIndicator {entity} />
              <span class="entity-name"
                >{#each highlightName(entity.name, searchState.term) as part}{#if part.matched}<mark
                      >{part.text}</mark
                    >{:else}{part.text}{/if}{/each}</span
              >
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

  <footer class="app-footer">
    <span>Contains Parliamentary information.</span><a
      href="https://www.parliament.uk/site-information/copyright-parliament/open-parliament-licence/"
      target="_blank"
      rel="noreferrer">Open Parliament Licence</a
    >
  </footer>
</div>
