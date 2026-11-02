<script lang="ts">
  import type { PageData, ActionData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}`);
</script>

<svelte:head
  ><title>{data.conversation.subject || 'Conversation'} · Looski</title
  ></svelte:head
>
<header>
  <a href={`${root}/communications`}>← Inbox</a><span>{data.account?.name}</span
  >
</header>
<h1>{data.conversation.subject || 'Conversation'}</h1>
<p>{data.conversation.channel} · {data.conversation.provider_thread_id}</p>
{#if form?.message}<p role="alert" class="error">
    {form.message} <a href="?">Reload</a>
  </p>{/if}
{#if data.contact}<p>
    Contact: <a href={`${root}/contacts/${data.contact.id}`}
      >{data.contact.display_name}</a
    >
  </p>{/if}
<section aria-label="Message history">
  {#each data.messages.items as m}<article
      class:outbound={m.direction === 'outbound'}
    >
      <p class="muted">
        {m.direction === 'outbound'
          ? 'Sent by your team'
          : m.sender || 'Customer'} · {m.created_at}
      </p>
      <p class="body">{m.body}</p>
      <p class="muted">
        {m.state}{#if m.job_state === 'blocked' || m.job_state === 'exhausted' || m.job_state === 'cancelled'}
          · Delivery {m.job_state}{/if}
      </p>
    </article>{/each}
  {#if data.messages.next_cursor}<a href={`?after=${data.messages.next_cursor}`}
      >More messages →</a
    >{/if}
</section>
{#if data.account?.enabled && data.account.capabilities.send_text && !data.conversation.archived}
  <section>
    <h2>Reply</h2>
    <form method="POST" action="?/send">
      <input
        type="hidden"
        name="idempotency_key"
        value={form?.key ?? data.key}
      /><label
        >Message<textarea name="body" required maxlength="20000" rows="5"
          >{form?.body ?? ''}</textarea
        ></label
      >
      <p>
        This sends through {data.account.name} to this conversation’s participants.
      </p>
      <button>Send reply</button>
    </form>
  </section>
{:else}<p>
    Replies are unavailable while the account is paused or the conversation is
    archived.
  </p>{/if}
<section>
  <h2>Contact and history</h2>
  <form method="GET">
    <label>Find a contact<input name="search" value={data.search} /></label
    ><button class="secondary">Search</button>
  </form>
  <form method="POST" action="?/update">
    <input
      type="hidden"
      name="idempotency_key"
      value={data.key + '-update'}
    /><input
      type="hidden"
      name="expected_version"
      value={data.conversation.version}
    />
    <label
      >Linked contact<select
        name="contact_id"
        value={data.conversation.contact_id ?? ''}
        ><option value="">No linked contact</option
        >{#if data.contact && !data.contacts.some((c) => c.id === data.contact?.id)}<option
            value={data.contact.id}>{data.contact.display_name}</option
          >{/if}{#each data.contacts as c}<option value={c.id}
            >{c.display_name}</option
          >{/each}</select
      ></label
    >
    <label
      ><input
        type="checkbox"
        name="archived"
        checked={data.conversation.archived}
      /> Archive conversation</label
    ><button class="secondary">Save</button>
  </form>
</section>

<style>
  article {
    padding: 1rem;
    margin-bottom: 0.75rem;
    border: 1px solid var(--border, #ddd);
    border-radius: 0.5rem;
  }
  .outbound {
    margin-left: 2rem;
  }
  .body {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
</style>
