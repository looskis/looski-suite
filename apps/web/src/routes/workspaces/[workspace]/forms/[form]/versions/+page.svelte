<script lang="ts">
  import FormDefinitionView from '$lib/components/FormDefinitionView.svelte';
  import type { PageData } from './$types';
  let { data }: { data: PageData } = $props();
</script>

<svelte:head><title>Versions · Forms · Looski</title></svelte:head>
<h1>Version history</h1>
<p>
  Each publish freezes a version. Later edits never change what earlier
  respondents answered.
</p>
<section>
  {#each data.versions as version, i}
    <details open={i === 0}>
      <summary
        >Version {version.number}{i === 0 ? ' (latest)' : ''} · {version.name} · published
        {version.published_at}</summary
      >
      <FormDefinitionView definition={version.definition} />
    </details>
  {:else}<p>This form has not been published yet.</p>{/each}
</section>
