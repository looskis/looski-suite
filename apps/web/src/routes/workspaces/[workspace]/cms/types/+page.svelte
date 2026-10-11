<script lang="ts">
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}/cms/types`);
</script>

<svelte:head><title>Content types · Looski</title></svelte:head>
<h1>Content types</h1>
<p>
  A content type sets the fields its entries have and gives them a public
  address, shared by nothing else in the workspace.
</p>
<section>
  <div class="toolbar">
    <h2>{data.archived ? 'Archived types' : 'Types'}</h2>
    <a href={data.archived ? '?' : '?archived=true'}
      >{data.archived ? 'Show active' : 'Show archived'}</a
    >
  </div>
  <div class="table-wrap">
    <table>
      <thead
        ><tr><th>Name</th><th>Fields</th><th>Published entries</th></tr></thead
      >
      <tbody>
        {#each data.page.items as item}
          <tr>
            <td><a href={`${root}/${item.id}`}>{item.name}</a></td>
            <td>{item.fields.length}</td>
            <td>{item.published_entries}</td>
          </tr>
        {:else}<tr><td colspan="3">No content types found.</td></tr>{/each}
      </tbody>
    </table>
  </div>
  {#if data.page.next_cursor}<a
      href={`?${new URLSearchParams({ archived: String(data.archived), after: data.page.next_cursor })}`}
      >Next page →</a
    >{/if}
</section>
<section>
  <h2>Create a content type</h2>
  {#if form?.message}<p role="alert">{form.message}</p>{/if}
  <form method="POST">
    <input type="hidden" name="idempotency_key" value={form?.key ?? data.key} />
    <label
      >Name<input
        name="name"
        required
        maxlength="120"
        value={form?.input?.name ?? ''}
      /></label
    >
    <label
      >Description (shown on the public list of pages)<textarea
        name="description"
        maxlength="2000"
        rows="2"
        value={form?.input?.description ?? ''}></textarea></label
    >
    <button>Create content type</button>
  </form>
</section>
