<script lang="ts">
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}/knowledge`);
  const collection = $derived(data.collection);
  const values = $derived(form?.input ?? collection);
</script>

<svelte:head><title>{collection.name} · Looski</title></svelte:head>
<header><a href={root}>← Knowledge base</a></header>
<h1>{collection.name}</h1>
<p>{collection.description}</p>
<p>
  {collection.restricted
    ? 'Restricted: only members listed below can find or read these documents.'
    : 'Open to everyone in the workspace with knowledge base read access.'}
  {#if collection.archived}This collection is archived: it stays readable, is
    left out of search and cannot be edited.{/if}
</p>
{#if form?.message}<p role="alert">{form.message}</p>{/if}
<section>
  <div class="toolbar">
    <h2>{data.archived ? 'Archived documents' : 'Documents'}</h2>
    {#if !collection.archived}<a
        href={`${root}/documents/new?collection=${collection.id}`}
        >New document</a
      >{/if}
    <a href={data.archived ? '?' : '?archived=true'}
      >{data.archived ? 'Show active' : 'Show archived'}</a
    >
  </div>
  <div class="table-wrap">
    <table>
      <thead
        ><tr><th>Title</th><th>Kind</th><th>Revision</th><th>Updated</th></tr
        ></thead
      >
      <tbody>
        {#each data.documents.items as item (item.id)}
          <tr>
            <td><a href={`${root}/documents/${item.id}`}>{item.title}</a></td>
            <td>{item.kind === 'sop' ? 'SOP' : 'Article'}</td>
            <td>{item.revision_number}</td>
            <td>{item.updated_at.slice(0, 16)}</td>
          </tr>
        {:else}<tr><td colspan="4">No documents found.</td></tr>{/each}
      </tbody>
    </table>
  </div>
  {#if data.documents.next_cursor}<a
      href={`?${new URLSearchParams({ archived: String(data.archived), after: data.documents.next_cursor })}`}
      >Next page →</a
    >{/if}
</section>
<section>
  <h2>Members</h2>
  <p>
    Listing grants nothing without a knowledge base role. Removal applies to the
    member's next request.
  </p>
  <div class="table-wrap">
    <table>
      <thead><tr><th>Name</th><th>State</th><th></th></tr></thead>
      <tbody>
        {#each data.members as member (member.id)}
          <tr>
            <td>{member.display_name}</td>
            <td>{member.active ? 'Active' : 'Inactive in this workspace'}</td>
            <td>
              {#if data.canManage}
                <form method="POST" action="?/remove" class="inline">
                  <input
                    type="hidden"
                    name="idempotency_key"
                    value={`${data.key}:${member.id}`}
                  />
                  <input type="hidden" name="membership_id" value={member.id} />
                  <button class="secondary">Remove</button>
                </form>
              {/if}
            </td>
          </tr>
        {:else}<tr><td colspan="3">No members listed.</td></tr>{/each}
      </tbody>
    </table>
  </div>
  {#if data.canManage && data.candidates.length}
    <form method="POST" action="?/add" class="inline">
      <input type="hidden" name="idempotency_key" value={`${data.key}:add`} />
      <label
        >Workspace member<select name="membership_id">
          {#each data.candidates as candidate (candidate.id)}<option
              value={candidate.id}>{candidate.display_name}</option
            >{/each}
        </select></label
      >
      <button>Add member</button>
    </form>
  {/if}
</section>
{#if data.canManage}
  {#if !collection.archived}
    <section>
      <h2>Settings</h2>
      <form method="POST" action="?/save">
        <input
          type="hidden"
          name="idempotency_key"
          value={form?.key ?? `${data.key}:save`}
        />
        <input
          type="hidden"
          name="expected_version"
          value={collection.version}
        />
        <label
          >Name<input
            name="name"
            required
            maxlength="120"
            value={values.name}
          /></label
        >
        <label
          >Description<textarea
            name="description"
            maxlength="2000"
            rows="2"
            value={values.description}></textarea></label
        >
        <label class="check"
          ><input
            type="checkbox"
            name="restricted"
            checked={values.restricted}
          />Restricted to listed members. Turning this off opens every document
          here, and its history, to the workspace.</label
        >
        <button>Save collection</button>
      </form>
    </section>
  {/if}
  <section>
    <h2>{collection.archived ? 'Restore' : 'Archive'}</h2>
    <form method="POST" action="?/archive">
      <input
        type="hidden"
        name="idempotency_key"
        value={`${data.key}:archive`}
      />
      <input type="hidden" name="expected_version" value={collection.version} />
      <input
        type="hidden"
        name="archived"
        value={String(!collection.archived)}
      />
      <button class="secondary"
        >{collection.archived
          ? 'Restore collection'
          : 'Archive collection'}</button
      >
    </form>
  </section>
{/if}
