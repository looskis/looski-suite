<script lang="ts">
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}/helpdesk`);
</script>

<svelte:head><title>Helpdesk · Looski</title></svelte:head>
<header><span>{data.workspace.name}</span></header>
<h1>Helpdesk</h1>
<p>Keep customer requests, replies and the next step together.</p>
{#if form?.message}<p role="alert">{form.message}</p>{/if}
{#if data.tickets}
  <section>
    <div class="toolbar">
      <h2>{data.archived ? 'Archived tickets' : 'Tickets'}</h2>
      <a href={data.archived ? '?' : '?archived=true'}
        >{data.archived ? 'Show active' : 'Show archived'}</a
      >
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
    <div class="table-wrap">
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
              ><td>{ticket.status}</td><td
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
{#if data.module}<section>
    <h2>Application selection</h2>
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
  </section>{/if}
