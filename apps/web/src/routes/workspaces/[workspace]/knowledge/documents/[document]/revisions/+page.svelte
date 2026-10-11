<script lang="ts">
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const root = $derived(
    `/workspaces/${data.workspace.id}/knowledge/documents/${data.document.id}`,
  );
  const frozen = $derived(
    data.document.archived || data.document.collection_archived,
  );
  const changeLabel = {
    created: 'Created',
    edited: 'Edited',
    moved: 'Moved',
    restored: 'Restored',
  };
</script>

<svelte:head><title>Revisions · Looski</title></svelte:head>
<header><a href={root}>← {data.document.title}</a></header>
<h1>Revisions</h1>
<p>
  Every saved change is kept. Restoring copies a revision into a new one and
  changes nothing in this history. Revisions written in a collection you cannot
  reach are not listed, so numbers may skip.
</p>
{#if form?.message}<p role="alert">{form.message}</p>{/if}
<div class="table-wrap">
  <table>
    <thead
      ><tr
        ><th>Revision</th><th>Title</th><th>Change</th><th>By</th><th>Saved</th
        ><th></th></tr
      ></thead
    >
    <tbody>
      {#each data.page.items as item (item.id)}
        <tr>
          <td><a href={`${root}/revisions/${item.id}`}>{item.number}</a></td>
          <td>{item.title}</td>
          <td
            >{changeLabel[item.change]}{#if item.restored_from_number}
              from {item.restored_from_number}{/if}</td
          >
          <td>{item.created_by_name}</td>
          <td>{item.created_at.slice(0, 19)}</td>
          <td>
            {#if !frozen && item.number !== data.document.revision_number}
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
                  value={data.document.version}
                />
                <button class="secondary">Restore</button>
              </form>
            {/if}
          </td>
        </tr>
      {:else}<tr><td colspan="6">No revisions you can read.</td></tr>{/each}
    </tbody>
  </table>
</div>
{#if data.page.next_cursor}<a
    href={`?${new URLSearchParams({ after: data.page.next_cursor })}`}
    >Next page →</a
  >{/if}
