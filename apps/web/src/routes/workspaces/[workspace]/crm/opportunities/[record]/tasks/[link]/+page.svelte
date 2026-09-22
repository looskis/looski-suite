<script lang="ts">
  let { data } = $props();
</script>

<header>
  <a
    href={`/workspaces/${data.workspace.id}/crm/${data.sales}/${data.id}/tasks`}
    >← Task links</a
  >
</header>
<h1>{data.history.link.task.title}</h1>
<p>{data.history.link.ended ? 'Ended link' : 'Active link'}</p>
<p>
  <a
    href={`/workspaces/${data.workspace.id}/tasks/${data.history.link.task.id}`}
    >Open task and task history</a
  >
</p>
<section>
  <h2>Link history</h2>
  <ol>
    {#each data.history.items as change}<li>
        {change.ended ? 'Link ended' : 'Link active'} · {change.occurred_at}
      </li>{/each}
  </ol>
  {#if data.history.next_cursor}<a href={`?after=${data.history.next_cursor}`}
      >More history →</a
    >{/if}
</section>
