<script lang="ts">
  let { data } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}`);
  const parent = $derived(
    `${root}/${data.page.anchor_kind === 'contact' ? 'contacts' : `crm/${data.page.anchor_kind === 'deal' ? 'deals' : 'opportunities'}`}/${data.page.anchor_id}`,
  );
  const query = $derived(
    `anchor_kind=${data.page.anchor_kind}&anchor_id=${data.page.anchor_id}`,
  );
</script>

<header>
  <a href={parent}>← {data.page.anchor_name}</a><a
    href={`${root}/crm/activities?${query}`}>Record activity</a
  >
</header>
<h1>Timeline · {data.page.anchor_name}</h1>
<p>
  Newest recorded changes first. Each activity and task requires its own read
  access. Task history includes work before and after its link to this record;
  ending a link preserves the history.
</p>
<a href={`?${query}`}>Refresh timeline</a>
<ol>
  {#each data.page.items as item}<li>
      <time datetime={item.recorded_at}>{item.recorded_at}</time>
      {#if item.event.kind === 'activity'}
        {@const event = item.event.detail}
        <h2>
          <a href={`${root}/crm/activities/${event.snapshot.id}`}
            >{event.snapshot.subject}</a
          >
        </h2>
        <p>
          {event.snapshot.type_name} · revision {event.snapshot.version}{event
            .snapshot.archived
            ? ' · Archived'
            : ''} · happened {event.snapshot.occurred_at}
        </p>
        <p class="body">{event.snapshot.body}</p>
      {:else if item.event.kind === 'appointment'}
        <p>
          <a href={`${root}/scheduling/${item.event.detail.appointment.id}`}
            >{item.event.detail.appointment.name}</a
          >
          · {item.event.detail.action} · {item.event.detail.starts_at}
        </p>
        {#if item.event.detail.appointment.intake}<details>
            <summary>Intake answers</summary>
            <dl>
              {#each Object.entries(item.event.detail.appointment.intake.answers) as [key, value]}<dt
                >
                  {key}
                </dt>
                <dd>
                  {typeof value === 'string' ? value : JSON.stringify(value)}
                </dd>{/each}
            </dl>
          </details>{/if}
      {:else if item.event.kind === 'ticket'}
        <h2>
          <a href={`${root}/helpdesk/${item.event.detail.ticket_id}`}
            >{item.event.detail.subject}</a
          >
        </h2>
        <p>Support · {item.event.detail.action.replaceAll('_', ' ')}</p>
      {:else if item.event.kind === 'transition'}
        <h2>
          {data.page.anchor_kind === 'deal'
            ? 'Deal status'
            : 'Opportunity stage'} changed
        </h2>
        <p>
          {item.event.detail.from_label ?? 'Created'} → {item.event.detail
            .to_label}
        </p>
      {:else if item.event.kind === 'task'}
        {@const event = item.event.detail}
        <h2>
          <a href={`${root}/tasks/${event.snapshot.id}`}
            >{event.snapshot.title}</a
          >
        </h2>
        <p>
          Task · {event.snapshot.status.replaceAll('_', ' ')} · revision {event
            .snapshot.version}{event.snapshot.archived
            ? ' · Archived'
            : ''}{event.snapshot.due_on
            ? ` · due ${event.snapshot.due_on}`
            : ''}
        </p>
        <p class="body">{event.snapshot.notes}</p>
      {:else}
        <h2>Task link {item.event.detail.ended ? 'ended' : 'active'}</h2>
        <a href={`${root}/tasks/${item.event.detail.task_id}`}
          >{item.event.detail.task_title}</a
        >
      {/if}
    </li>{:else}<li>No visible events yet.</li>{/each}
</ol>
{#if data.page.next_cursor}<a
    href={`?${query}&after=${encodeURIComponent(data.page.next_cursor)}`}
    >Earlier events →</a
  >{/if}

<style>
  .body {
    white-space: pre-wrap;
  }
  li {
    padding-block: 1rem;
    border-bottom: 1px solid #ddd;
  }
  ol {
    list-style: none;
    padding: 0;
  }
</style>
