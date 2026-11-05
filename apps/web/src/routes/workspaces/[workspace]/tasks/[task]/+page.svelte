<script lang="ts">
  import RecordDrawer from '$lib/components/ui/RecordDrawer.svelte';
  import { shortDate } from '$lib/format';
  import { page } from '$app/state';
  import TaskFields from '$lib/components/TaskFields.svelte';
  let { data, form } = $props();
</script>

<header>
  <a href={`/workspaces/${data.workspace.id}/tasks`}>← Tasks</a><span
    >{data.task.archived
      ? 'Archived'
      : data.task.status.replaceAll('_', ' ')}</span
  >
</header>
<div class="page-heading">
  <h1>{data.task.title}</h1>
  <div class="page-actions">
    <RecordDrawer
      message={form?.message}
      title="Edit task"
      secondary
      error={Boolean(form?.draft || form?.archiveDraft) ||
        page.url.searchParams.has('parent')}
    >
      <form method="POST" action="?/save">
        <input
          type="hidden"
          name="idempotency_key"
          value={form?.draft?.key ?? data.key}
        />
        <input
          type="hidden"
          name="expected_version"
          value={form?.draft?.expected_version ?? data.task.version}
        />
        <TaskFields fields={form?.draft ?? data.fields} /><button
          disabled={data.task.archived}>Save task</button
        >
      </form>
      {#if data.task.archived}<p>Restore this task before editing.</p>{/if}
      <form method="POST" action="?/archive">
        <input
          type="hidden"
          name="idempotency_key"
          value={form?.archiveDraft?.key ?? data.archiveKey}
        />
        <input
          type="hidden"
          name="expected_version"
          value={form?.archiveDraft?.expected_version ?? data.task.version}
        />
        <input
          type="hidden"
          name="archived"
          value={form?.archiveDraft?.archived ?? String(!data.task.archived)}
        />
        <button class="secondary"
          >{data.task.archived ? 'Restore task' : 'Archive task'}</button
        >
      </form>
    </RecordDrawer>
  </div>
</div>
<dl class="record-summary">
  <div>
    <dt>Status</dt>
    <dd>{data.task.status.replaceAll('_', ' ')}</dd>
  </div>
  <div>
    <dt>Due date</dt>
    <dd>{data.task.due_on ?? 'Unscheduled'}</dd>
  </div>
  <div>
    <dt>Type</dt>
    <dd>{data.task.type_name ?? 'General'}</dd>
  </div>
  <div>
    <dt>Updated</dt>
    <dd>{shortDate(data.task.updated_at)}</dd>
  </div>
</dl>
<section>
  <h2>Notes</h2>
  <p style="white-space: pre-wrap">{data.task.notes || 'No notes added.'}</p>
</section>

{#if data.task.completed_at}<p>Completed {data.task.completed_at}</p>{/if}
{#if form?.message}<p role="alert" class="error">{form.message}</p>{/if}
{#if form?.draft || form?.archiveDraft}<p>
    <a href={`/workspaces/${data.workspace.id}/tasks/${data.task.id}`}
      >Discard draft and reload saved values</a
    >
  </p>{/if}

<section>
  <h2>History</h2>
  <ol>
    {#each data.history.items as change}<li>
        <strong>{change.snapshot.title}</strong> — {change.snapshot.status.replaceAll(
          '_',
          ' ',
        )}{change.snapshot.archived ? ' · archived' : ''}
        <p>Version {change.snapshot.version} · {change.occurred_at}</p>
        <p>{change.snapshot.notes}</p>
        <p>Type: {change.snapshot.type_name ?? 'Unspecified'}</p>
        <p>
          Due: {change.snapshot.due_on ?? 'Unscheduled'} · Assignee: {change
            .snapshot.assigned_membership_id}
        </p>
      </li>{/each}
  </ol>
  {#if data.history.next_cursor}<a
      href={`?history_after=${data.history.next_cursor}`}>More history →</a
    >{/if}
</section>

<section>
  <h2>Task configuration</h2>
  <a href={`/workspaces/${data.workspace.id}/tasks/types`}>Task types</a> ·
  <a href={`/workspaces/${data.workspace.id}/tasks/templates`}
    >Reusable templates</a
  >
</section>
