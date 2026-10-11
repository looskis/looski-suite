<script lang="ts">
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}`);
</script>

<svelte:head><title>{data.ticket.subject} · Helpdesk</title></svelte:head>
<a href={`${root}/helpdesk`}>← Helpdesk</a>
<h1>{data.ticket.subject}</h1>
<p>
  {data.ticket.requester_name} · {data.ticket.status}{data.ticket.archived
    ? ' · Archived'
    : ''}
</p>
{#if form?.message}<p role="alert">{form.message}</p>{/if}
<section>
  <h2>Conversation</h2>
  <ol class="conversation">
    {#each data.messages.items as message}<li
        class:private-note={message.kind === 'note'}
      >
        <strong
          >{message.kind === 'note'
            ? 'Internal note — staff only'
            : message.kind === 'customer'
              ? 'Customer'
              : 'Support reply'}</strong
        ><small>{message.created_at}</small>
        <p class="body">{message.body}</p>
      </li>{/each}
  </ol>
  {#if data.messages.next_cursor}<a href={`?after=${data.messages.next_cursor}`}
      >More messages →</a
    >{/if}
  {#if !data.ticket.archived}<form method="POST" action="?/reply">
      <input
        type="hidden"
        name="idempotency_key"
        value={form?.key ?? data.key}
      /><label
        >Message type<select
          name="internal"
          value={String(form?.input?.internal ?? false)}
          ><option value="false">Public reply — visible to customer</option
          ><option value="true">Internal note — staff only</option></select
        ></label
      ><label
        >Message<textarea
          name="body"
          rows="5"
          maxlength="20000"
          required
          value={form?.input?.body ?? ''}></textarea></label
      ><button>Send message</button>
    </form>{/if}
</section>
<section>
  <h2>Ticket details</h2>
  <form method="POST" action="?/update">
    <input type="hidden" name="idempotency_key" value={data.key} /><input
      type="hidden"
      name="expected_version"
      value={data.ticket.version}
    /><label
      >Status<select name="status" value={data.ticket.status}
        ><option value="open">Open</option><option value="waiting"
          >Waiting for customer</option
        ><option value="resolved">Resolved</option></select
      ></label
    ><label
      >Assigned to<select
        name="assignee_id"
        value={data.ticket.assignee_id ?? ''}
        ><option value="">Unassigned</option
        >{#each data.directory as member}<option value={member.id}
            >{member.display_name}</option
          >{/each}</select
      ></label
    ><label
      >Lifecycle<select name="archived" value={String(data.ticket.archived)}
        ><option value="false">Active</option><option value="true"
          >Archived — customer access paused</option
        ></select
      ></label
    >
    {#if data.ticket.customer_revoked}<p>
        Customer access has been permanently revoked.
      </p>{:else}<label
        ><input type="checkbox" name="revoke" /> Permanently revoke the customer's
        access code</label
      >{/if}<button>Save details</button>
  </form>
</section>
<section>
  <h2>Staff reference links</h2>
  <p>
    These links and article previews are visible only to staff with permission.
    They are never included in customer history.
  </p>
  {#if data.ticket.contact_id}<p>
      <a href={`${root}/contacts/${data.ticket.contact_id}`}>Linked contact</a>
      ·
      <a
        href={`${root}/crm/timeline?anchor_kind=contact&anchor_id=${data.ticket.contact_id}`}
        >Contact timeline</a
      >
    </p>{/if}
  <ul>
    {#each data.ticket.articles as article}<li>
        {#if article.preview}<a
            href={`${root}/knowledge/documents/${article.id}`}
            >{article.preview.title}</a
          >{:else}Article unavailable{/if}
      </li>{/each}
  </ul>
  <form method="GET" class="inline">
    <label
      >Find a contact<input
        name="contact_search"
        value={data.contactSearch}
      /></label
    ><label>Find an article<input name="q" value={data.q} /></label><button
      class="secondary">Find</button
    >
  </form>
  {#if !data.ticket.archived}<form method="POST" action="?/links">
      <input type="hidden" name="idempotency_key" value={data.key} /><input
        type="hidden"
        name="expected_version"
        value={data.ticket.version}
      /><label
        >Contact<select name="contact_id" value={data.ticket.contact_id ?? ''}
          ><option value="">No linked contact</option
          >{#if data.contact && !data.contacts.some((c) => c.id === data.contact?.id)}<option
              value={data.contact.id}>{data.contact.display_name}</option
            >{/if}{#each data.contacts as contact}<option value={contact.id}
              >{contact.display_name}</option
            >{/each}</select
        ></label
      >
      <fieldset>
        <legend>Knowledge articles — staff only</legend
        >{#each data.ticket.articles ?? [] as article}<label
            ><input
              type="checkbox"
              name="article_ids"
              value={article.id}
              checked
            />{article.preview?.title ??
              'Unavailable article (uncheck to remove)'}</label
          >{/each}{#each data.articles.filter((a) => !(data.ticket.articles ?? []).some((link) => link.id === a.id)) as article}<label
            ><input
              type="checkbox"
              name="article_ids"
              value={article.id}
            />{article.title}</label
          >{/each}
      </fieldset>
      <button class="secondary">Save links</button>
    </form>{/if}
</section>

<style>
  h1 {
    overflow-wrap: anywhere;
  }
  .conversation {
    list-style: none;
    padding: 0;
  }
  .conversation li {
    padding: 1rem;
    border-bottom: 1px solid #dce5de;
  }
  .private-note {
    background: #fff8e5;
    border-left: 3px solid #d5a83f;
  }
  .body {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  small {
    display: block;
    color: #68786e;
    margin-top: 0.3rem;
  }
</style>
