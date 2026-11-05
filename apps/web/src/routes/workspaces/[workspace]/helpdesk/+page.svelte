<script lang="ts">
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}/helpdesk`);
</script>

<svelte:head><title>Helpdesk · Looski</title></svelte:head>

<div class="page-heading">
  <h1>Helpdesk</h1>
  {#if data.settings?.accepting}<a
      class="action-link"
      href={`${data.origin}/support/new/${data.settings.public_key}`}
      >Open support page ↗</a
    >{/if}
</div>
{#if form?.message}<p role="alert">{form.message}</p>{/if}
{#if data.tickets}
  <section class="index-list">
    <div class="toolbar">
      <nav class="view-tabs" aria-label="Ticket views">
        <a href="?" aria-current={!data.archived ? 'page' : undefined}
          >All tickets</a
        ><a
          href="?archived=true"
          aria-current={data.archived ? 'page' : undefined}>Archived</a
        >
      </nav>
    </div>
    <form method="GET" class="inline">
      <input
        type="hidden"
        name="archived"
        value={String(data.archived)}
      /><label
        >Status<select name="status" value={data.status}
          ><option value="">All statuses</option><option value="open"
            >Open</option
          ><option value="waiting">Waiting</option><option value="resolved"
            >Resolved</option
          ></select
        ></label
      ><button class="secondary">Filter</button>
    </form>
    <div class="record-table">
      <table>
        <thead
          ><tr
            ><th>Subject</th><th>Requester</th><th>Status</th><th>Assignment</th
            ></tr
          ></thead
        ><tbody
          >{#each data.tickets.items as ticket}<tr
              ><td><a href={`${root}/${ticket.id}`}>{ticket.subject}</a></td><td
                >{ticket.requester_name}</td
              ><td><span class="status-badge">{ticket.status}</span></td><td
                >{ticket.assignee_id ? 'Assigned' : 'Unassigned'}</td
              ></tr
            >{:else}<tr
              ><td colspan="4"
                >No tickets yet. Open intake below and share the support page.</td
              ></tr
            >{/each}</tbody
        >
      </table>
    </div>
    {#if data.tickets.next_cursor}<a
        href={`?${new URLSearchParams({ archived: String(data.archived), status: data.status, after: data.tickets.next_cursor })}`}
        >Next page →</a
      >{/if}
  </section>
  {#if data.settings}<section>
      <h2>Customer intake</h2>
      <p>
        {data.settings.accepting
          ? 'Your support page is accepting new tickets.'
          : 'New ticket intake is closed. Existing customers can still reply.'}
      </p>
      {#if data.settings.accepting}<p>
          <a href={`${data.origin}/support/new/${data.settings.public_key}`}
            >{data.origin}/support/new/{data.settings.public_key}</a
          >
        </p>{/if}
      <form method="POST" action="?/configure">
        <input type="hidden" name="idempotency_key" value={data.key} /><input
          type="hidden"
          name="expected_version"
          value={data.settings.version}
        /><input
          type="hidden"
          name="accepting"
          value={String(!data.settings.accepting)}
        /><button class="secondary"
          >{data.settings.accepting ? 'Close intake' : 'Open intake'}</button
        >
      </form>
      <p>
        Customers receive a private access code for their ticket. Anyone with
        that code can read and reply to it. Email delivery is not connected.
      </p>
    </section>{/if}
{:else}<section>
    <h2>Helpdesk is turned off</h2>
    <p>
      Tickets and messages are retained. Customer access resumes when Helpdesk
      is enabled.
    </p>
  </section>{/if}
{#if data.module}<details class="creation-panel" open={Boolean(form?.message)}>
    <summary>Application settings</summary>
    <div class="creation-content">
      <form method="POST" action="?/module">
        <input type="hidden" name="idempotency_key" value={data.key} /><input
          type="hidden"
          name="expected_version"
          value={data.module.version}
        /><input
          type="hidden"
          name="enabled"
          value={String(!data.module.enabled)}
        /><button class="secondary"
          >{data.module.enabled
            ? 'Turn off Helpdesk'
            : 'Turn on Helpdesk'}</button
        >
      </form>
    </div>
  </details>{/if}
