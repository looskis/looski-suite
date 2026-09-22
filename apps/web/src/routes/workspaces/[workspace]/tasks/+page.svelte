<script lang="ts">
  import TaskFields from '$lib/components/TaskFields.svelte';
  let { data, form } = $props();
</script>

<header>
  <a href="/workspaces">← Workspaces</a><a href={`?archived=${!data.archived}`}
    >{data.archived ? 'View active' : 'View archived'}</a
  >
</header>
<h1>Tasks</h1>
<p>Actionable work for {data.workspace.name}.</p>
{#if form?.message}<p role="alert" class="error">{form.message}</p>{/if}
<form method="GET" class="inline">
  <input type="hidden" name="archived" value={String(data.archived)} />
  <label>Search<input name="search" value={data.search} /></label>
  <label
    >Assignee membership ID<input
      name="assigned_membership_id"
      value={data.assignedMembershipId}
    /></label
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
<section>
  <div class="table-wrap">
    <table>
      <thead><tr><th>Task</th><th>Status</th><th>Due</th></tr></thead><tbody
        >{#each data.page.items as task}<tr
            ><td
              ><a href={`/workspaces/${data.workspace.id}/tasks/${task.id}`}
                >{task.title}</a
              ></td
            ><td>{task.status.replaceAll('_', ' ')}</td><td
              >{task.due_on ?? '—'}</td
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
<section>
  <h2>Create task</h2>
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
</section>

<section>
  <h2>Task configuration</h2>
  <a href={`/workspaces/${data.workspace.id}/tasks/types`}>Task types</a> ·
  <a href={`/workspaces/${data.workspace.id}/tasks/templates`}
    >Reusable templates</a
  >
</section>
