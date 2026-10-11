<script lang="ts">
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}/cms/media`);
</script>

<svelte:head><title>Media · Looski</title></svelte:head>
<h1>Media</h1>
<p>
  Media comes from Files and stays private there. A public page can show an item
  only while a published entry uses it. PNG, JPEG, GIF, WebP and PDF only.
</p>
{#if form?.message}<p role="alert">{form.message}</p>{/if}
<section>
  <div class="toolbar">
    <h2>{data.archived ? 'Archived media' : 'Library'}</h2>
    <a href={data.archived ? '?' : '?archived=true'}
      >{data.archived ? 'Show active' : 'Show archived'}</a
    >
  </div>
  <div class="table-wrap">
    <table>
      <thead><tr><th>Title</th><th>Type</th><th>File</th></tr></thead>
      <tbody>
        {#each data.page.items as item}
          <tr>
            <td><a href={`${root}/${item.id}`}>{item.title}</a></td>
            <td>{item.media_type}</td>
            <td
              >{item.filename}{item.file_archived
                ? ' (archived in Files)'
                : ''}</td
            >
          </tr>
        {:else}<tr><td colspan="3">No media yet.</td></tr>{/each}
      </tbody>
    </table>
  </div>
  {#if data.page.next_cursor}<a
      href={`?${new URLSearchParams({ archived: String(data.archived), after: data.page.next_cursor })}`}
      >Next page →</a
    >{/if}
</section>
<section>
  <h2>Upload</h2>
  <form method="POST" action="?/upload" enctype="multipart/form-data">
    <input type="hidden" name="idempotency_key" value={form?.key ?? data.key} />
    <input
      type="hidden"
      name="file_key"
      value={form?.fileKey ?? data.fileKey}
    />
    <label
      >File<input
        type="file"
        name="file"
        required
        accept="image/png,image/jpeg,image/gif,image/webp,application/pdf"
      /></label
    >
    <label
      >Title<input
        name="title"
        maxlength="200"
        value={form?.input?.title ?? ''}
      /></label
    >
    <label
      >Description for people who cannot see it<input
        name="alt_text"
        maxlength="1000"
        value={form?.input?.alt_text ?? ''}
      /></label
    >
    <button>Upload</button>
  </form>
</section>
{#if data.files.length}
  <section>
    <h2>Use a file already in Files</h2>
    <form method="POST" action="?/register">
      <input
        type="hidden"
        name="idempotency_key"
        value={form?.key ?? data.key}
      />
      <label
        >File<select name="file_id" required>
          {#each data.files as file}
            <option value={file.id}>{file.filename} ({file.media_type})</option>
          {/each}
        </select></label
      >
      <label>Title<input name="title" required maxlength="200" /></label>
      <label
        >Description for people who cannot see it<input
          name="alt_text"
          maxlength="1000"
        /></label
      >
      <button class="secondary">Add to library</button>
    </form>
  </section>
{/if}
