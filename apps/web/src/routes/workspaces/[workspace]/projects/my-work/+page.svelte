<script lang="ts">
  import { taskStatusLabel } from '$lib/project-labels';
  import type { PageData } from './$types';
  let { data }: { data: PageData } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}/projects`);
</script>

<svelte:head><title>My work · Looski</title></svelte:head>
<header><a href={root}>← Projects</a></header>
<h1>My work</h1>
<p>
  Tasks assigned to you in projects you can open. Today is {data.work.today}.
</p>
<div class="toolbar">
  <a href={data.finished ? '?' : '?finished=true'}
    >{data.finished ? 'Hide finished' : 'Show finished'}</a
  >
</div>
<div class="table-wrap">
  <table>
    <thead
      ><tr><th>Task</th><th>Project</th><th>Status</th><th>Deadline</th></tr
      ></thead
    >
    <tbody>
      {#each data.work.items as item}
        <tr>
          <td
            ><a href={`${root}/${item.project_id}/cards/${item.card_id}`}
              >{item.task.title}</a
            ></td
          >
          <td><a href={`${root}/${item.project_id}`}>{item.project_name}</a></td
          >
          <td>{taskStatusLabel(item.task.status)}</td>
          <td
            >{item.task.due_on ?? '—'}{#if item.overdue}
              <strong>Overdue</strong>{/if}</td
          >
        </tr>
      {:else}<tr><td colspan="4">Nothing assigned to you here.</td></tr>{/each}
    </tbody>
  </table>
</div>
{#if data.work.next_cursor}<a
    href={`?${new URLSearchParams({ finished: String(data.finished), after: data.work.next_cursor })}`}
    >Next page →</a
  >{/if}
