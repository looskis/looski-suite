<script lang="ts">
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}/knowledge`);
  const changeLabel = {
    created: 'Created',
    edited: 'Edited',
    moved: 'Moved',
    restored: 'Restored',
  };
  let suggestions = $state<{ id: string; title: string }[]>([]);
  // A convenience on top of a plain search form, which works without scripting.
  async function suggest(event: Event) {
    const q = (event.currentTarget as HTMLInputElement).value.trim();
    if (q.length < 2) {
      suggestions = [];
      return;
    }
    const response = await fetch(
      `${root}/suggest?${new URLSearchParams({ q })}`,
    );
    if (response.ok) suggestions = (await response.json()).items;
  }
</script>

<svelte:head><title>Knowledge base · Looski</title></svelte:head>

<div class="page-heading">
  <h1>Knowledge base</h1>
  {#if data.collections}<a class="action-link" href={`${root}/documents/new`}
      >New document</a
    >{/if}
</div>
{#if form?.message && !form?.input}<p role="alert">{form.message}</p>{/if}
{#if data.collections}
  <section class="index-list">
    <form method="GET" class="inline" role="search">
      <label
        >Find documents<input
          name="q"
          value={data.q}
          maxlength="200"
          list="knowledge-suggestions"
          autocomplete="off"
          oninput={suggest}
        /></label
      >
      <datalist id="knowledge-suggestions">
        {#each suggestions as item (item.id)}<option value={item.title}
          ></option>{/each}
      </datalist>
      <button class="secondary">Search</button>
    </form>
    {#if data.searchError}<p role="alert">{data.searchError}</p>{/if}
    {#if data.results}
      <p>
        {data.results.total}
        {data.results.total === 1 ? 'document' : 'documents'} found.
      </p>
      <ul class="results">
        {#each data.results.items as hit (hit.id)}
          <li>
            <a href={`${root}/documents/${hit.id}`}>{hit.title}</a>
            <small
              >{hit.collection_name} · {hit.kind === 'sop'
                ? 'SOP'
                : 'Article'}</small
            >
            <!-- Typed parts only: matched words are marked by structure, never markup. -->
            <p>
              {#each hit.snippet as part}{#if part.matched}<mark
                    >{part.text}</mark
                  >{:else}{part.text}{/if}{/each}
            </p>
          </li>
        {/each}
      </ul>
      {#if data.results.next_cursor}<a
          href={`?${new URLSearchParams({ q: data.q, after: data.results.next_cursor })}`}
          >Next page →</a
        >{/if}
    {/if}
  </section>
  <section class="index-list">
    <div class="toolbar">
      <h2>{data.archived ? 'Archived collections' : 'Collections'}</h2>

      <a href={data.archived ? '?' : '?archived=true'}
        >{data.archived ? 'Show active' : 'Show archived'}</a
      >
    </div>
    <div class="record-table">
      <table>
        <thead
          ><tr><th>Name</th><th>Access</th><th>Documents</th><th>Listed</th></tr
          ></thead
        >
        <tbody>
          {#each data.collections.items as item (item.id)}
            <tr>
              <td><a href={`${root}/collections/${item.id}`}>{item.name}</a></td
              >
              <td>{item.restricted ? 'Restricted' : 'Workspace'}</td>
              <td>{item.document_count}</td>
              <td>{item.is_member ? 'Yes' : 'No'}</td>
            </tr>
          {:else}<tr><td colspan="4">No collections found.</td></tr>{/each}
        </tbody>
      </table>
    </div>
    {#if data.collections.next_cursor}<a
        href={`?${new URLSearchParams({ archived: String(data.archived), collections_after: data.collections.next_cursor })}`}
        >Next page →</a
      >{/if}
  </section>
  {#if data.activity?.length}
    <section>
      <h2>Recent changes</h2>
      <ul class="results">
        {#each data.activity as item (item.revision_id)}
          <li>
            <a href={`${root}/documents/${item.document_id}`}>{item.title}</a>
            <small
              >{changeLabel[item.change]} by {item.actor_name} · {item.collection_name}
              · {item.created_at.slice(0, 16)}</small
            >
          </li>
        {/each}
      </ul>
    </section>
  {/if}
  <details class="creation-panel" open={Boolean(form?.message)}>
    <summary>Create a collection</summary>
    <div class="creation-content">
      {#if form?.message && form?.input}<p role="alert">{form.message}</p>{/if}
      <form method="POST" action="?/createCollection">
        <input
          type="hidden"
          name="idempotency_key"
          value={form?.key ?? data.key}
        />
        <label
          >Name<input
            name="name"
            required
            maxlength="120"
            value={form?.input?.name ?? ''}
          /></label
        >
        <label
          >Description<textarea
            name="description"
            maxlength="2000"
            rows="2"
            value={form?.input?.description ?? ''}></textarea></label
        >
        <label class="check"
          ><input
            type="checkbox"
            name="restricted"
            checked={form?.input?.restricted ?? false}
          />Restricted: only members listed on this collection can find or read
          it</label
        >
        <button>Create collection</button>
      </form>
    </div>
  </details>
{:else}
  <section>
    <h2>The knowledge base is turned off</h2>
    <p>
      Collections, documents and revisions are retained, and come back when it
      is turned on again.
    </p>
  </section>
{/if}
{#if data.module}
  <details class="creation-panel" open={Boolean(form?.message)}>
    <summary>Application settings</summary>
    <div class="creation-content">
      <p>
        The knowledge base is {data.module.enabled ? 'on' : 'off'} for this workspace.
      </p>
      <form method="POST" action="?/module">
        <input type="hidden" name="idempotency_key" value={data.key} />
        <input
          type="hidden"
          name="expected_version"
          value={data.module.version}
        />
        <input
          type="hidden"
          name="enabled"
          value={String(!data.module.enabled)}
        />
        <button class={data.module.enabled ? 'secondary' : ''}
          >{data.module.enabled
            ? 'Turn off the knowledge base'
            : 'Turn on the knowledge base'}</button
        >
      </form>
    </div>
  </details>
{/if}

<style>
  .results {
    list-style: none;
    padding: 0;
    margin: 0;
  }
  .results li {
    padding: var(--space-3) 0;
    border-top: 1px solid var(--border-default);
  }
  .results small {
    display: block;
    color: var(--text-secondary);
  }
  .results p {
    margin: var(--space-1-5) 0 0;
    font-size: var(--font-size-13);
    color: var(--text-secondary);
  }
</style>
