<script lang="ts">
  import AppShell from "../../AppShell.svelte";
  import {
    ResultRow,
    SectionHeading,
    Notice,
    IconIndicator,
  } from "../../lib/ui";
  import { updateMonth, updateDate } from "../../lib/updates";
  import type { PageData } from "./$types";
  let { data }: { data: PageData } = $props();
</script>

<svelte:head><title>Updates | Exposed</title></svelte:head>
<AppShell target="updates" current="updates">
  <main id="updates" tabindex="-1">
    <h1>Updates</h1>
    {#if data.updates}
      <p class="month">
        <time datetime={data.updates.monthStart.slice(0, 7)}
          >{updateMonth(data.updates.monthStart)}</time
        >
      </p>
      <div class="sections">
        {#each [{ id: "members", title: "MPs", rows: data.updates.members, accent: "member" }, { id: "companies", title: "Companies", rows: data.updates.companies, accent: "funder" }] as section (section.id)}
          <section
            aria-labelledby={`${section.id}-heading`}
            style={`--accent: var(--accent-${section.accent})`}
          >
            <SectionHeading
              id={`${section.id}-heading`}
              title={section.title}
            />
            {#if section.rows.length}
              <ul class="results-list" aria-label={`${section.title} updates`}>
                {#each section.rows as row (row.id)}
                  <ResultRow>
                    <IconIndicator
                      icon={section.id === "members"
                        ? "parliament"
                        : "building"}
                      label={section.id === "members" ? "MP" : "Company"}
                    />
                    <div class="entity-text">
                      <a
                        class="entity-name"
                        href={`/${section.id === "members" ? "members" : "funders"}/${row.id}`}
                        >{row.name}</a
                      >
                      <span class="entity-alias"
                        >{row.declarationCount} declaration{row.declarationCount ===
                        1
                          ? ""
                          : "s"}</span
                      >
                    </div>
                    <time class="registered" datetime={row.latestRegistration}
                      >{updateDate(row.latestRegistration)}</time
                    >
                  </ResultRow>
                {/each}
              </ul>
            {:else}<p class="empty">No declarations found.</p>{/if}
          </section>
        {/each}
      </div>
    {:else}<Notice role="alert">{data.error}</Notice>{/if}
  </main>
</AppShell>

<style>
  .month {
    margin: var(--space-2) 0 var(--space-6);
    color: var(--muted);
    font-size: var(--text-heading);
  }
  .sections {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-8);
  }
  .entity-name {
    color: var(--accent);
  }
  .registered {
    flex: none;
    color: var(--muted);
    font-size: var(--text-small);
    text-align: right;
  }
  .empty {
    color: var(--muted);
  }
  @media (max-width: 800px) {
    .sections {
      grid-template-columns: 1fr;
    }
  }
  @media (max-width: 480px) {
    .registered {
      grid-column: 2;
      text-align: left;
    }
  }
</style>
