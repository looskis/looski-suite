<script lang="ts">
  import { answerText } from '$lib/form-definition';
  import RecipeRunStatus from '$lib/components/RecipeRunStatus.svelte';
  import type { PageData } from './$types';
  let { data }: { data: PageData } = $props();
  const fields = $derived(
    (data.version?.definition.fields ?? []).filter(
      (f) => f.kind !== 'statement',
    ),
  );
  // Without definition access, or for keys unknown to the version, show keys.
  const others = $derived(
    Object.entries(data.submission.answers).filter(
      ([key]) => !fields.some((f) => f.key === key),
    ),
  );
</script>

<svelte:head><title>Submission · Forms · Looski</title></svelte:head>
<h1>Submission</h1>
<p>
  Submitted {data.submission.submitted_at} against version {data.submission
    .version_number}{data.version ? ` (${data.version.name})` : ''}.
</p>
<section>
  <h2>Answers</h2>
  <dl>
    {#each fields as field}
      {@const answer = answerText(field, data.submission.answers[field.key])}
      <dt>{field.label}</dt>
      <dd class:empty={!answer}>{answer || 'No answer'}</dd>
    {/each}
    {#each others as [key, value]}
      <dt><code>{key}</code></dt>
      <dd>{answerText(undefined, value)}</dd>
    {/each}
  </dl>
</section>
{#if data.submission.recipe_run}
  <section>
    <h2>CRM recipe</h2>
    <RecipeRunStatus
      run={data.submission.recipe_run}
      workspace={data.workspace.id}
    />
    <p>
      <a
        href={`/workspaces/${data.workspace.id}/forms/${data.submission.form_id}/recipe`}
        >Open recipe runs to retry</a
      >
    </p>
  </section>
{/if}

<style>
  dt {
    font-size: 13px;
    font-weight: 550;
    margin-top: 16px;
  }
  dd {
    margin: 4px 0 0;
    white-space: pre-wrap;
  }
  dd.empty {
    color: var(--muted);
  }
</style>
