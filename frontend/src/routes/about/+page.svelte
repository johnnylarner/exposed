<script lang="ts">
  import AppShell from "../../AppShell.svelte";
  import About from "../../about.md";
  import { Button, Field, Notice, Textarea } from "../../lib/ui";
  import { feedbackLimits } from "../../lib/feedback";
  import type { ActionData } from "./$types";

  let { form }: { form: ActionData } = $props();
</script>

<svelte:head>
  <title>About · Exposed</title>
  <meta
    name="description"
    content="About Exposed and the team behind the project."
  />
</svelte:head>

<AppShell target="about" label="about" current="about">
  <main id="about" tabindex="-1">
    <article aria-labelledby="about-heading">
      <div class="prose"><About /></div>
      <form method="POST" aria-labelledby="feedback">
        <Field
          label="Email"
          name="email"
          type="email"
          autocomplete="email"
          required
          maxlength={feedbackLimits.email}
          value={form?.values?.email ?? ""}
          error={form?.kind === "invalid" ? form.errors.email : undefined}
        />
        <Textarea
          label="Message"
          name="message"
          required
          rows={6}
          maxlength={feedbackLimits.message}
          value={form?.values?.message ?? ""}
          error={form?.kind === "invalid" ? form.errors.message : undefined}
        />
        {#if form?.kind === "success"}
          <Notice role="status">Thank you. Your feedback has been sent.</Notice>
        {:else if form?.kind === "unavailable"}
          <Notice tone="warning" role="alert"
            >Feedback is temporarily unavailable. Please try again later.</Notice
          >
        {:else if form?.kind === "failed"}
          <Notice tone="warning" role="alert"
            >Your feedback could not be sent. Please try again.</Notice
          >
        {/if}
        <div><Button type="submit">Send feedback</Button></div>
      </form>
    </article>
  </main>
</AppShell>

<style>
  form {
    display: grid;
    gap: var(--space-4);
  }
  main {
    width: min(100%, 74ch);
    min-width: 0;
  }
  article {
    font-size: var(--text-body);
    line-height: 1.85;
    overflow-wrap: anywhere;
  }
  .prose :global(h1) {
    font-size: calc(var(--text-heading) * 2);
    line-height: 1.2;
    margin: 0 0 var(--space-6);
  }
  .prose :global(h2) {
    font-size: var(--text-heading);
    line-height: 1.35;
    margin: 0 0 var(--space-4);
  }
  .prose :global(p) {
    margin: 0 0 var(--space-4);
  }
  .prose :global(.article-intro p) {
    color: var(--muted);
  }
  .prose :global(section) {
    margin-top: var(--space-8);
  }
</style>
