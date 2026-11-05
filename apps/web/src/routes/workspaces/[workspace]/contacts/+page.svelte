<script lang="ts">
  import { page } from '$app/state';
  import RecordDrawer from '$lib/components/ui/RecordDrawer.svelte';
  import RecordViewControls from '$lib/components/RecordViewControls.svelte';
  import RecordTable from '$lib/components/RecordTable.svelte';
  import ContactFields from '$lib/components/ContactFields.svelte';
  import type { PageData, ActionData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  function nextPage() {
    const query = new URLSearchParams(page.url.searchParams);
    query.set('after', data.page.next_cursor ?? '');
    return `?${query}`;
  }
</script>

<svelte:head
  ><title>Contacts · {data.workspace.name} · Looski</title></svelte:head
>
<div class="page-heading">
  <h1>
    Contacts<span class="count"
      >{data.page.items.length}{data.page.next_cursor ? ' +' : ''} shown</span
    >
  </h1>
  <div class="page-actions">
    <RecordDrawer
      title="New contact"
      error={Boolean(form?.message && form?.input)}
      >{#if form?.message}<p role="alert" class="error">{form.message}</p>{/if}
      <form method="POST" action="?/create">
        <input
          type="hidden"
          name="idempotency_key"
          value={form?.idempotencyKey ?? data.idempotencyKey}
        /><ContactFields contact={form?.input} fields={data.fields} /><button
          >Create contact</button
        >
      </form></RecordDrawer
    >
  </div>
</div>
{#if form?.message && !form?.input}<p role="alert" class="error">
    {form.message}
  </p>{/if}
<RecordViewControls
  fields={data.fields}
  views={data.views}
  config={data.config}
  viewId={data.viewId}
  viewKey={data.viewKey}
  kind="contacts"
  workspace={data.workspace.id}
  search={data.search}
  tagId={data.tagId}
/>
<RecordTable
  items={data.page.items}
  fields={data.fields}
  config={data.config}
  root={`/workspaces/${data.workspace.id}/contacts`}
/>
<div class="list-footer">
  <span>{data.page.items.length} contacts on this page</span
  >{#if data.page.next_cursor}<a href={nextPage()}>Next page →</a>{/if}
</div>
