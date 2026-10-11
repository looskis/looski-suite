<script lang="ts">
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const root = $derived(
    `/workspaces/${data.workspace.id}/cms/entries/${data.entry.id}`,
  );
</script>

<svelte:head><title>Revisions · Looski</title></svelte:head>
<header><a href={root}>← Back to editing</a></header>
<h1>Revisions</h1>
<p>
  Every saved change is kept. Restoring copies a revision into the draft as a
  new revision; the public page changes only when you publish.
</p>
{#if form?.message}<p role="alert">{form.message}</p>{/if}
<div class="table-wrap">
  <table>
    <thead
      ><tr
        ><th>Revision</th><th>Title</th><th>Saved</th><th>Status</th><th
        ></th></tr
      ></thead
    >
    <tbody>
      {#each data.page.items as item}
        <tr>
          <td><a href={`${root}/revisions/${item.id}`}>{item.number}</a></td>
          <td>{item.title}</td>
          <td>{item.created_at.slice(0, 19)}</td>
          <td>
            {#if data.entry.published?.revision_id === item.id}Live{:else if item.last_published_at}Published
              {item.last_published_at.slice(0, 10)}{/if}
            {#if item.restored_from_number}(restored from {item.restored_from_number}){/if}
          </td>
          <td>
            <form method="POST" action="?/restore">
              <input
                type="hidden"
                name="idempotency_key"
                value={`${data.key}-${item.number}`}
              />
              <input type="hidden" name="revision_id" value={item.id} />
              <input
                type="hidden"
                name="expected_version"
                value={data.entry.version}
              />
              <button class="secondary">Restore</button>
            </form>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>
{#if data.page.next_cursor}<a href={`?after=${data.page.next_cursor}`}
    >Older revisions →</a
  >{/if}
