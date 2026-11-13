<script lang="ts">
  import type { PageData } from './$types';
  let { data }: { data: PageData } = $props();
  const labels: Record<string, string> = {
    submitted: 'Submitted for review',
    changes_requested: 'Changes requested',
    published: 'Published',
  };
</script>

<svelte:head><title>Writing updates · Looski</title></svelte:head>
<h1>Writing updates</h1>
<p>The latest 50 submissions, feedback, and publications you can access.</p>
{#each data.events as event}
  <section>
    <h2>
      <a href={`/workspaces/${data.workspace.id}/cms/entries/${event.entry_id}`}
        >{event.title}</a
      >
    </h2>
    <p>{labels[event.kind]} · {event.created_at.slice(0, 16)}</p>
    {#if event.note}<p>{event.note}</p>{/if}
  </section>
{:else}<p>No updates yet.</p>{/each}
