<script lang="ts">
  import MoneyValue from '$lib/components/MoneyValue.svelte';
  import RecordDrawer from '$lib/components/ui/RecordDrawer.svelte';
  import { shortDate } from '$lib/format';
  import { page } from '$app/state';
  import TagPanel from '$lib/components/TagPanel.svelte';
  import CrmRecordFields from '$lib/components/CrmRecordFields.svelte';
  import CrmWorkflowPicker from '$lib/components/CrmWorkflowPicker.svelte';
  let { data, form } = $props();
  const workflow = $derived({
    ...data.workflow,
    parent: form?.draft?.parent_id ?? data.workflow.parent,
    state: form?.draft?.state_id ?? data.workflow.state,
  });
</script>

<header>
  <a href={`/workspaces/${data.workspace.id}/crm/opportunities`}
    >← Opportunities</a
  ><span>Version {data.record.version}</span>
</header>
<a
  href={`/workspaces/${data.workspace.id}/crm/timeline?anchor_kind=opportunity&anchor_id=${data.record.id}`}
  >Timeline</a
>
<div class="page-heading">
  <h1>{data.record.title}</h1>
  <div class="page-actions">
    {#if !data.record.archived}<RecordDrawer
        message={form?.message}
        title="Edit opportunity"
        secondary
        error={Boolean(form?.draft) || page.url.searchParams.has('parent')}
      >
        <CrmWorkflowPicker isDeal={false} {workflow} />
        <form method="POST" action="?/save">
          <input
            type="hidden"
            name="idempotency_key"
            value={form?.draft?.key ?? data.key}
          /><input
            type="hidden"
            name="expected_version"
            value={form?.draft?.expected_version ?? data.record.version}
          /><CrmRecordFields
            isDeal={false}
            fields={form?.draft ?? data.fields}
            {workflow}
          /><button>Save opportunity</button>
        </form>
      </RecordDrawer>{/if}
  </div>
</div>
<dl class="record-summary">
  <div>
    <dt>Value</dt>
    <dd>
      <MoneyValue
        amount={data.record.expected_value}
        currency={data.record.currency_code}
        billingFrequency={data.record.billing_frequency}
      />
    </dd>
  </div>
  <div>
    <dt>Expected close</dt>
    <dd>{data.record.expected_close_on ?? '—'}</dd>
  </div>
  <div>
    <dt>Updated</dt>
    <dd>{shortDate(data.record.updated_at)}</dd>
  </div>
</dl>
<section>
  <h2>Notes</h2>
  <p style="white-space: pre-wrap">{data.record.notes || 'No notes added.'}</p>
</section>
<p>
  {data.record.pipeline_name} · {data.record.stage_name}{data.record.archived
    ? ' · Archived'
    : ''}
</p>
<p>
  <a
    href={`/workspaces/${data.workspace.id}/crm/deals?origin=${data.record.id}`}
    >Create a deal from this opportunity →</a
  >
</p>
{#if form?.message}<p class="error" role="alert">
    {form.message}
    <a
      href={`/workspaces/${data.workspace.id}/crm/opportunities/${data.record.id}`}
      >Discard draft and reload saved values</a
    >
  </p>{/if}

<section>
  <h2>Stage history</h2>
  <ol>
    {#each data.history.items as transition}<li>
        <strong
          >{transition.from_label ?? 'Created'} → {transition.to_label}</strong
        >
        <p class="muted">
          {transition.occurred_at}{transition.to_outcome
            ? ` · ${transition.to_outcome}`
            : ''}
        </p>
      </li>{/each}
  </ol>
  {#if data.history.next_cursor}<p>
      <a href={`?history_after=${data.history.next_cursor}`}>More history →</a>
    </p>{/if}
</section>
<section>
  <h2>{data.record.archived ? 'Restore' : 'Archive'}</h2>
  <p>
    Keep this record and its history. Archiving does not change its sales
    outcome.
  </p>
  <form method="POST" action="?/archive">
    <input
      type="hidden"
      name="idempotency_key"
      value={data.archive_key}
    /><input
      type="hidden"
      name="expected_version"
      value={data.record.version}
    /><input
      type="hidden"
      name="archived"
      value={!data.record.archived ? 'true' : 'false'}
    /><button class="secondary"
      >{data.record.archived ? 'Restore' : 'Archive'} opportunity</button
    >
  </form>
</section>

<TagPanel
  draft={form?.tagDraft}
  panel={data.tags}
  workspace={data.workspace.id}
  version={data.record.version}
/>

<section>
  <h2>Participants</h2>
  <p>
    <a
      href={`/workspaces/${data.workspace.id}/crm/opportunities/${data.record.id}/participants/contacts`}
      >Contact participants</a
    >
    ·
    <a
      href={`/workspaces/${data.workspace.id}/crm/opportunities/${data.record.id}/participants/organizations`}
      >Organization participants</a
    >
  </p>
</section>

<section>
  <h2>Tasks</h2>
  <a
    href={`/workspaces/${data.workspace.id}/crm/opportunities/${data.record.id}/tasks`}
    >Manage follow-up tasks</a
  >
</section>

<section>
  <h2>Activity log</h2>
  <a
    href={`/workspaces/${data.workspace.id}/crm/activities?anchor_kind=opportunity&anchor_id=${data.record.id}`}
    >View and record activities</a
  >
</section>
