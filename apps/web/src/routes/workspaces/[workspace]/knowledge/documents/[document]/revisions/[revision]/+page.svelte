<script lang="ts">
  import RichText from '$lib/components/RichText.svelte';
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const root = $derived(
    `/workspaces/${data.workspace.id}/knowledge/documents/${data.document.id}`,
  );
  const frozen = $derived(
    data.document.archived || data.document.collection_archived,
  );
</script>

<svelte:head
  ><title>Revision {data.revision.number} · Looski</title></svelte:head
>
<header><a href={`${root}/revisions`}>← Revisions</a></header>
<h1>{data.revision.title}</h1>
<p>
  Revision {data.revision.number}, saved {data.revision.created_at.slice(0, 19)} by
  {data.revision.created_by_name} in {data.revision.collection_name}.
</p>
{#if form?.message}<p role="alert">{form.message}</p>{/if}
<section><RichText blocks={data.revision.blocks} /></section>
{#if !frozen && data.revision.number !== data.document.revision_number}
  <form method="POST">
    <input type="hidden" name="idempotency_key" value={data.key} />
    <input
      type="hidden"
      name="expected_version"
      value={data.document.version}
    />
    <button>Restore as a new revision</button>
  </form>
{/if}
