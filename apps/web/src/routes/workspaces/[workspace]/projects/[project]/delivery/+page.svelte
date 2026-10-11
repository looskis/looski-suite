<script lang="ts">
  import { taskStatuses, taskStatusLabel } from '$lib/project-labels';
  import type { PageData } from './$types';
  let { data }: { data: PageData } = $props();
  const root = $derived(
    `/workspaces/${data.workspace.id}/projects/${data.delivery.project.id}`,
  );
  const s = $derived(data.delivery.summary);
  const done = $derived(
    s.total ? Math.round((100 * s.completed) / s.total) : 0,
  );
  const stats = $derived([
    ['Tasks', s.total],
    ['Open', s.open],
    ['In progress', s.in_progress],
    ['Completed', s.completed],
    ['Overdue', s.overdue],
    ['Due this week', s.due_soon],
    ['No deadline', s.unscheduled],
    ['After project deadline', s.past_project_deadline],
  ] as const);
</script>

<svelte:head
  ><title>Delivery · {data.delivery.project.name} · Looski</title></svelte:head
>
<h1>Delivery: {data.delivery.project.name}</h1>
<p>
  {done}% of the tasks you can see are complete. Today is {data.delivery
    .today}{#if data.delivery.project.due_on}; the project is due {data.delivery
      .project.due_on}{/if}.
</p>
<dl class="stats">
  {#each stats as [label, value]}<div>
      <dt>{label}</dt>
      <dd>{value}</dd>
    </div>{/each}
</dl>
<section>
  <h2>Workload</h2>
  <div class="table-wrap">
    <table>
      <thead
        ><tr><th>Assignee</th><th>Unfinished</th><th>Overdue</th></tr></thead
      >
      <tbody>
        {#each data.delivery.workload as load}
          <tr>
            <td
              ><a href={`?assignee=${load.membership_id}`}
                >{load.display_name ?? 'Outside this project'}</a
              ></td
            >
            <td>{load.unfinished}</td>
            <td>{load.overdue}</td>
          </tr>
        {:else}<tr><td colspan="3">No unfinished work you can see.</td></tr
          >{/each}
      </tbody>
    </table>
  </div>
</section>
<section>
  <h2>Tasks by deadline</h2>
  <form method="GET" class="inline">
    <label
      >Status<select name="status">
        <option value="">Any</option>
        {#each taskStatuses as status}<option
            value={status.value}
            selected={data.filters.status === status.value}
            >{status.label}</option
          >{/each}
      </select></label
    >
    <label
      >Deadline<select name="due">
        <option value="">Any</option>
        <option value="overdue" selected={data.filters.due === 'overdue'}
          >Overdue</option
        >
        <option value="soon" selected={data.filters.due === 'soon'}
          >Due this week</option
        >
        <option
          value="unscheduled"
          selected={data.filters.due === 'unscheduled'}>No deadline</option
        >
      </select></label
    >
    {#if data.filters.assigned_membership_id}<input
        type="hidden"
        name="assignee"
        value={data.filters.assigned_membership_id}
      />{/if}
    <button class="secondary">Apply</button>
    <a href="?">Clear</a>
  </form>
  <div class="table-wrap">
    <table>
      <thead
        ><tr
          ><th>Task</th><th>Column</th><th>Assignee</th><th>Status</th><th
            >Deadline</th
          ></tr
        ></thead
      >
      <tbody>
        {#each data.delivery.items as item}
          <tr>
            <td
              ><a href={`${root}/cards/${item.card_id}`}>{item.task.title}</a
              ></td
            >
            <td>{item.column_name}</td>
            <td>{item.assignee_name ?? 'Outside this project'}</td>
            <td>{taskStatusLabel(item.task.status)}</td>
            <td
              >{item.task.due_on ?? '—'}{#if item.overdue}
                <strong>Overdue</strong>{/if}{#if item.past_project_deadline}
                <strong>After project deadline</strong>{/if}</td
            >
          </tr>
        {:else}<tr><td colspan="5">No tasks match.</td></tr>{/each}
      </tbody>
    </table>
  </div>
</section>

<style>
  .stats {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    margin: 16px 0 24px;
  }
  .stats div {
    border: 1px solid var(--line, #ddd);
    border-radius: 6px;
    padding: 8px 14px;
    min-width: 110px;
  }
  dt {
    font-size: 12px;
    color: var(--muted);
  }
  dd {
    margin: 0;
    font-size: 20px;
    font-weight: 650;
  }
</style>
