<script lang="ts">
  import RecordProperties from '$lib/components/RecordProperties.svelte';
  import RecordDrawer from '$lib/components/ui/RecordDrawer.svelte';
  import { shortDate } from '$lib/format';
  import TagPanel from '$lib/components/TagPanel.svelte';
  import OrganizationFields from '$lib/components/OrganizationFields.svelte';
  import type { PageData, ActionData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
</script>

<svelte:head
  ><title>{data.organization.name} · Organizations · Looski</title></svelte:head
>
<header>
  <a href={`/workspaces/${data.workspace.id}/organizations`}>← Organizations</a>
</header>
<div class="page-heading">
  <h1>
    {data.organization.name}<span class="status-badge"
      >{data.organization.archived ? 'Archived' : 'Active'}</span
    >
  </h1>
  <div class="page-actions">
    <RecordDrawer
      message={form?.message}
      title="Edit organization"
      label="Edit organization"
      secondary
      error={Boolean(form?.input)}
    >
      <form method="POST" action="?/edit">
        <input
          type="hidden"
          name="idempotency_key"
          value={form?.idempotencyKey ?? data.editKey}
        />
        <input
          type="hidden"
          name="expected_version"
          value={form?.expectedVersion ?? data.organization.version}
        />
        <OrganizationFields
          fields={data.fields}
          organization={form?.input ?? data.organization}
        /><button>Save changes</button>
      </form>
    </RecordDrawer>
  </div>
</div>
<dl class="record-summary">
  <div>
    <dt>Website</dt>
    <dd>{data.organization.website_url || '—'}</dd>
  </div>
  <div>
    <dt>Created</dt>
    <dd>{shortDate(data.organization.created_at)}</dd>
  </div>
  <div>
    <dt>Updated</dt>
    <dd>{shortDate(data.organization.updated_at)}</dd>
  </div>
</dl>
<div class="record-layout">
  <div class="record-main">
    <section>
      <h2>Notes</h2>
      <p class="record-notes">{data.organization.notes || 'No notes added.'}</p>
    </section>

    {#if form?.message}<p role="alert" class="error">
        {form.message} <a href={``}>Reload current record</a>
      </p>{/if}
  </div>
  <aside class="record-side">
    <RecordProperties
      fields={data.fields}
      properties={data.organization.properties}
    />
    <TagPanel
      draft={form?.tagDraft}
      panel={data.tags}
      workspace={data.workspace.id}
      version={data.organization.version}
    />
    <section>
      <h2>
        {data.organization.archived
          ? 'Restore organization'
          : 'Archive organization'}
      </h2>
      <p>
        Archiving keeps the organization and its history. You can restore it
        later.
      </p>
      <form method="POST" action="?/archive">
        <input
          type="hidden"
          name="idempotency_key"
          value={data.archiveKey}
        /><input
          type="hidden"
          name="expected_version"
          value={data.organization.version}
        /><input
          type="hidden"
          name="archived"
          value={String(!data.organization.archived)}
        /><button class="secondary"
          >{data.organization.archived
            ? 'Restore organization'
            : 'Archive organization'}</button
        >
      </form>
    </section>
  </aside>
</div>

<style>
  .record-notes {
    white-space: pre-wrap;
  }
  .record-main > section:first-child {
    margin-top: 0;
  }
  .record-side :global(section:first-child) {
    margin-top: 0;
  }
</style>
