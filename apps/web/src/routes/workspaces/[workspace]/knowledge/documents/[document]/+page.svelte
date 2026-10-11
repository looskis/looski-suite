<script lang="ts">
  import RichText from '$lib/components/RichText.svelte';
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}/knowledge`);
  const document = $derived(data.document);
  const frozen = $derived(document.archived || document.collection_archived);
</script>

<svelte:head><title>{document.title} · Looski</title></svelte:head>
<header>
  <a href={`${root}/collections/${document.collection_id}`}
    >← {document.collection_name}</a
  >
  {#if !frozen}<a href={`${root}/documents/${document.id}/edit`}>Edit</a>{/if}
  <a href={`${root}/documents/${document.id}/revisions`}>Revisions</a>
</header>
<h1>{document.title}</h1>
<p>
  {document.kind === 'sop' ? 'Standard operating procedure' : 'Article'} · revision
  {document.revision_number} · updated {document.updated_at.slice(0, 16)} by {document.updated_by_name}
  {#if document.collection_restricted}· restricted collection{/if}
  {#if document.archived}· archived{/if}
</p>
{#if form?.message}<p role="alert">{form.message}</p>{/if}
<section><RichText blocks={document.blocks} /></section>
{#if document.linked.length}
  <section>
    <h2>Linked documents</h2>
    <ul>
      {#each document.linked as link (link.id)}
        <li>
          {#if link.preview}
            <a href={`${root}/documents/${link.id}`}>{link.preview.title}</a>
            <small>{link.preview.collection_name}</small>
            <p>{link.preview.excerpt}</p>
          {:else}
            A linked document you cannot open.
          {/if}
        </li>
      {/each}
    </ul>
  </section>
{/if}
<section>
  <h2>Link to this document</h2>
  <p>Paste this into another document:</p>
  <pre>{`[${document.title}](${root}/documents/${document.id})`}</pre>
</section>
{#if !frozen && data.targets.length}
  <section>
    <h2>Move</h2>
    <p>
      Readers of the new collection can read this at once and those of the
      current one lose it. Earlier revisions stay visible only to those who can
      reach the collection they were written in.
    </p>
    <form method="POST" action="?/move" class="inline">
      <input type="hidden" name="idempotency_key" value={`${data.key}:move`} />
      <input type="hidden" name="expected_version" value={document.version} />
      <label
        >Collection<select name="collection_id">
          {#each data.targets as item (item.id)}<option value={item.id}
              >{item.name}{item.restricted ? ' (restricted)' : ''}</option
            >{/each}
        </select></label
      >
      <button class="secondary">Move document</button>
    </form>
  </section>
{/if}
{#if !document.collection_archived}
  <section>
    <h2>{document.archived ? 'Restore' : 'Archive'}</h2>
    <form method="POST" action="?/archive">
      <input
        type="hidden"
        name="idempotency_key"
        value={`${data.key}:archive`}
      />
      <input type="hidden" name="expected_version" value={document.version} />
      <input type="hidden" name="archived" value={String(!document.archived)} />
      <button class="secondary"
        >{document.archived ? 'Restore document' : 'Archive document'}</button
      >
    </form>
  </section>
{/if}
