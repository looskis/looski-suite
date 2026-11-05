<script lang="ts">
  import { shortDate } from '$lib/format';
  import RecordDrawer from '$lib/components/ui/RecordDrawer.svelte';
  import TaskFields from '$lib/components/TaskFields.svelte';
  let { data, form } = $props();
</script>

<div class="page-heading">
  <h1>Tasks <span class="count">{data.page.items.length} shown</span></h1>
  <div class="page-actions">
    <RecordDrawer
      message={form?.message}
      title="New task"
      error={Boolean(form?.message)}
    >
      {#if form?.draft}<p>
          <a href={`/workspaces/${data.workspace.id}/tasks`}
            >Discard draft and reload</a
          >
        </p>{/if}
      <form method="POST">
        <input
          type="hidden"
          name="idempotency_key"
          value={form?.draft?.key ?? data.key}
        /><TaskFields fields={form?.draft ?? data.fields} creating /><button
          >Create task</button
        >
      </form>
    </RecordDrawer>
  </div>
</div>
{#if form?.message}<p role="alert" class="error">{form.message}</p>{/if}
<nav class="view-tabs" aria-label="Task views">
  <a href="?" aria-current={!data.archived ? 'page' : undefined}>All tasks</a><a
    href="?archived=true"
    aria-current={data.archived ? 'page' : undefined}>Archived</a
  >
</nav>
<form method="GET" class="task-filters">
  <input type="hidden" name="archived" value={String(data.archived)} />
  <label>Search<input name="search" value={data.search} /></label>
  <label
    >Assignee<select
      name="assigned_membership_id"
      value={data.assignedMembershipId}
      ><option value="">Anyone</option>{#each data.members as member}<option
          value={member.id}>{member.display_name}</option
        >{/each}</select
    ></label
  >
  <label
    >Status<select name="status" value={data.status}
      ><option value="">Any status</option><option value="open">Open</option
      ><option value="in_progress">In progress</option><option value="completed"
        >Completed</option
      ><option value="cancelled">Cancelled</option></select
    ></label
  >
  <label
    >Due on or before<input
      type="date"
      name="due_before"
      value={data.dueBefore}
    /></label
  ><button class="secondary">Filter</button>
</form>
<section class="index-list">
  <div class="record-table">
    <table>
      <thead
        ><tr
          ><th>Task</th><th>Type</th><th>Status</th><th>Due</th><th>Updated</th
          ></tr
        ></thead
      ><tbody
        >{#each data.page.items as task}<tr
            ><td
              ><a href={`/workspaces/${data.workspace.id}/tasks/${task.id}`}
                >{task.title}</a
              ></td
            ><td>{task.type_name ?? 'General'}</td><td
              ><span class="task-status" data-status={task.status}
                >{task.status.replaceAll('_', ' ')}</span
              ></td
            ><td
              class:overdue={task.due_on &&
                task.due_on < data.today &&
                !['completed', 'cancelled'].includes(task.status)}
              >{task.due_on ? shortDate(task.due_on) : '—'}</td
            ><td class="cell-date"
              >{data.members.find(
                (m: { id: string; display_name: string }) =>
                  m.id === task.assigned_membership_id,
              )?.display_name ?? '—'}</td
            ></tr
          >{/each}</tbody
      >
    </table>
  </div>
  {#if !data.page.items.length}<p>No tasks match these filters.</p>{/if}
  {#if data.page.next_cursor}<a
      href={`?after=${data.page.next_cursor}&search=${encodeURIComponent(data.search)}&status=${data.status}&archived=${data.archived}&due_before=${data.dueBefore}&assigned_membership_id=${data.assignedMembershipId}`}
      >More tasks →</a
    >{/if}
</section>

<div class="list-footer">
  <a href={`/workspaces/${data.workspace.id}/tasks/types`}>Task types</a><a
    href={`/workspaces/${data.workspace.id}/tasks/templates`}
    >Reusable templates</a
  >
</div>

<style>
  .task-filters {
    display: flex;
    align-items: end;
    flex-wrap: wrap;
    gap: 12px;
    padding: 18px 0;
  }
  .task-filters label {
    margin: 0;
    font-size: 10px;
    color: var(--text-secondary);
  }
  .task-filters input,
  .task-filters select {
    margin: 5px 0 0;
    width: auto;
    min-width: 140px;
  }
  .task-status {
    display: inline-flex;
    gap: 6px;
    align-items: center;
    font-size: 11px;
    text-transform: capitalize;
  }
  .task-status:before {
    content: '';
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #8e98a6;
  }
  .task-status[data-status='in_progress']:before {
    background: #3b82f6;
  }
  .task-status[data-status='completed']:before {
    background: #16a34a;
  }
  .task-status[data-status='cancelled'] {
    opacity: 0.6;
  }
  .overdue {
    color: var(--danger-text, #b42318);
    font-weight: 500;
  }
</style>
