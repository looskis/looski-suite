<script lang="ts">
  import { page } from '$app/state';
  import type { PageData } from './$types';
  let { data }: { data: PageData } = $props();
  const revision = $derived(data.revision);
</script>

<svelte:head><title>Revision {revision.number} · Looski</title></svelte:head>
<header>
  <a
    href={`/workspaces/${data.workspace.id}/cms/entries/${page.params.entry}/revisions`}
    >← Revisions</a
  >
</header>
<h1>Revision {revision.number}: {revision.title}</h1>
<p>Saved {revision.created_at.slice(0, 19)}. Address: /{revision.slug}</p>
<dl>
  {#each revision.fields as field (field.key)}
    <dt>{field.label}</dt>
    <dd style="white-space:pre-wrap">
      {revision.values[field.key] === undefined
        ? '—'
        : String(revision.values[field.key])}
    </dd>
  {/each}
</dl>
