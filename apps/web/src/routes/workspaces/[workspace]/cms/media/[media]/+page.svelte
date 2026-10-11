<script lang="ts">
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const content = $derived(
    `/workspaces/${data.workspace.id}/cms/media/${data.item.id}/content`,
  );
</script>

<svelte:head><title>{data.item.title} · Media · Looski</title></svelte:head>
<h1>{data.item.title}</h1>
{#if form?.message}<p role="alert">{form.message}</p>{/if}
{#if data.item.file_archived}<p>
    The file is archived in Files, so public pages do not show this item.
  </p>{/if}
{#if !data.item.file_archived}
  {#if data.item.media_type.startsWith('image/')}
    <img src={content} alt={data.item.alt_text} style="max-width:100%" />
  {:else}
    <p><a href={content}>Download {data.item.filename}</a></p>
  {/if}
{/if}
<p>
  From Files:
  <a href={`/workspaces/${data.workspace.id}/files/${data.item.file_id}`}
    >{data.item.filename}</a
  >
</p>
<section>
  <h2>Details</h2>
  <form method="POST" action="?/save">
    <input type="hidden" name="idempotency_key" value={form?.key ?? data.key} />
    <input type="hidden" name="expected_version" value={data.item.version} />
    <label
      >Title<input
        name="title"
        required
        maxlength="200"
        value={data.item.title}
      /></label
    >
    <label
      >Description for people who cannot see it<input
        name="alt_text"
        maxlength="1000"
        value={data.item.alt_text}
      /></label
    >
    <button>Save</button>
  </form>
</section>
<section>
  <h2>{data.item.archived ? 'Restore' : 'Archive'}</h2>
  {#if !data.item.archived}<p>
      Archiving removes this item from every public page at once. Pages regain
      it when it is restored.
    </p>{/if}
  <form method="POST" action="?/archive">
    <input type="hidden" name="idempotency_key" value={data.key} />
    <input type="hidden" name="expected_version" value={data.item.version} />
    <input
      type="hidden"
      name="archived"
      value={data.item.archived ? 'false' : 'true'}
    />
    <button class="secondary"
      >{data.item.archived ? 'Restore media' : 'Archive media'}</button
    >
  </form>
</section>
