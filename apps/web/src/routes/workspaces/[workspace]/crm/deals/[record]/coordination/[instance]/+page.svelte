<script lang="ts">
  let { data } = $props();
</script>

<header>
  <a
    href={`/workspaces/${data.workspace.id}/crm/deals/${data.instance.deal_id}/coordination`}
    >← Deal checklists</a
  >
</header>
<h1>{data.instance.template_name}</h1>
<p>Base date {data.instance.base_on} · Applied {data.instance.created_at}</p>
<p>Pinned revision: {data.instance.revision_id}</p>
<section>
  <h2>Visible work</h2>
  <p>
    Task permissions apply independently. Only work you can read appears here.
  </p>
  <ol>
    {#each data.instance.items as item}<li>
        <h3>{item.label}</h3>
        <p>
          <a href={`/workspaces/${data.workspace.id}/tasks/${item.task.id}`}
            >{item.task.title}</a
          >
          · {item.task.status.replaceAll('_', ' ')}{item.task.archived
            ? ' · archived'
            : ''}{item.link_ended ? ' · CRM link ended' : ''}
        </p>
        <p>
          Due: {item.task.due_on ?? 'Unscheduled'} · Assigned membership: {item
            .task.assigned_membership_id}
        </p>
        <p>{item.task.notes}</p>
        <a
          href={`/workspaces/${data.workspace.id}/crm/deals/${data.instance.deal_id}/tasks/${item.link_id}`}
          >Link history</a
        >
      </li>{:else}<li>No task details are currently available.</li>{/each}
  </ol>
</section>
