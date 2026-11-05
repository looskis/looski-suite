<script lang="ts">
  import { tick } from 'svelte';
  let history: HTMLDivElement;
  import { shortDate, messageTime } from '$lib/format';
  import type { PageData, ActionData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  $effect(() => {
    data.messages.items;
    tick().then(() => {
      if (history) history.scrollTop = history.scrollHeight;
    });
  });
  const root = $derived(`/workspaces/${data.workspace.id}`);
</script>

<svelte:head
  ><title>{data.conversation.subject || 'Conversation'} · Inbox · Looski</title
  ></svelte:head
>
<a class="back-link" href={`${root}/communications`}>← Inbox</a>
<div class="thread-header">
  <div class="thread-avatar" aria-hidden="true">
    {(data.contact?.display_name || data.conversation.subject || '?')
      .slice(0, 2)
      .toUpperCase()}
  </div>
  <div>
    <h1>
      {data.conversation.subject ||
        data.contact?.display_name ||
        'Conversation'}
    </h1>
    <p>
      {data.conversation.channel} · {data.account?.name ??
        'Connected account'}{data.conversation.archived ? ' · Archived' : ''}
    </p>
  </div>
</div>
{#if form?.message}<p role="alert" class="error">{form.message}</p>{/if}
<div class="thread-layout">
  <div class="thread-main">
    <div
      bind:this={history}
      class="message-history"
      aria-label="Message history"
    >
      {#if data.olderPage}<a class="message-pagination" href="?"
          >Latest messages →</a
        >{/if}
      {#if data.messages.next_cursor}<a
          class="message-pagination"
          href={`?after=${data.messages.next_cursor}`}>← Earlier messages</a
        >{/if}
      {#each data.messages.items as message, i}
        {#if i === 0 || message.created_at.slice(0, 10) !== data.messages.items[i - 1].created_at.slice(0, 10)}<div
            class="date-divider"
          >
            <span>{shortDate(message.created_at)}</span>
          </div>{/if}
        <article
          class="message"
          class:outbound={message.direction === 'outbound'}
        >
          <div class="message-author">
            {message.direction === 'outbound'
              ? 'Your team'
              : message.sender || data.contact?.display_name || 'Customer'}
          </div>
          <div class="message-bubble">{message.body}</div>
          <div class="message-meta">
            <time datetime={message.created_at}
              >{messageTime(message.created_at)}</time
            >{#if message.direction === 'outbound'}<span
                >{message.state}{#if ['blocked', 'exhausted', 'cancelled'].includes(message.job_state ?? '')}
                  · Delivery {message.job_state}{/if}</span
              >{/if}
          </div>
        </article>
      {:else}<div class="empty-state">
          No messages in this conversation.
        </div>{/each}
    </div>
    {#if data.account?.enabled && data.account.capabilities.send_text && !data.conversation.archived}<form
        class="composer"
        method="POST"
        action="?/send"
      >
        <input
          type="hidden"
          name="idempotency_key"
          value={form?.key ?? data.key}
        /><label class="sr-only" for="reply-message">Reply</label><textarea
          id="reply-message"
          name="body"
          required
          maxlength="20000"
          rows="3"
          placeholder="Write a reply…">{form?.body ?? ''}</textarea
        >
        <div class="composer-footer">
          <span>Send via {data.account.name}</span><button>Send reply ↗</button>
        </div>
      </form>{:else}<p class="reply-unavailable">
        Replies are unavailable for this account or archived conversation.
      </p>{/if}
  </div>
  <aside class="thread-sidebar">
    <h2>Details</h2>
    {#if data.contact}<a
        class="linked-contact"
        href={`${root}/contacts/${data.contact.id}`}
        >{data.contact.display_name} ↗</a
      >{/if}
    <dl>
      <dt>Channel</dt>
      <dd>{data.conversation.channel}</dd>
      <dt>Account</dt>
      <dd>{data.account?.name ?? '—'}</dd>
    </dl>
    <details open={Boolean(data.search)}>
      <summary>Contact &amp; archive</summary>
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
    </details>
  </aside>
</div>

<style>
  .thread-header {
    display: flex;
    align-items: center;
    gap: 14px;
    margin: 16px 0 24px;
  }
  .thread-header h1 {
    margin: 0;
    font-size: 20px;
  }
  .thread-header p {
    font-size: 12px;
    color: var(--text-secondary);
    margin: 5px 0 0;
  }
  .thread-avatar {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    border-radius: 14px;
    background: var(--surface-hover);
    border: 1px solid var(--border-default);
    font-size: 13px;
  }
  .thread-layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 260px;
    border: 1px solid var(--border-default);
    border-radius: 12px;
    overflow: hidden;
    background: var(--surface-panel);
  }
  .thread-main {
    min-width: 0;
  }
  .message-history {
    padding: 24px;
    height: calc(100dvh - 370px);
    min-height: 240px;
    max-height: 680px;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .date-divider {
    display: flex;
    align-items: center;
    gap: 14px;
    margin: 6px 0;
    font-size: 10px;
    color: var(--text-secondary);
  }
  .date-divider:before,
  .date-divider:after {
    content: '';
    height: 1px;
    background: var(--border-default);
    flex: 1;
  }
  .message {
    max-width: 82%;
    align-self: flex-start;
    margin: 0;
    padding: 0;
    border: 0;
    background: none;
    box-shadow: none;
  }
  .message.outbound {
    align-self: flex-end;
  }
  .message-author {
    font-size: 10px;
    color: var(--text-secondary);
    margin: 0 2px 6px;
  }
  .outbound .message-author {
    text-align: right;
  }
  .message-bubble {
    padding: 12px 16px;
    border-radius: 14px 14px 14px 4px;
    background: var(--surface-hover);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    font-size: 13px;
    line-height: 1.6;
  }
  .outbound .message-bubble {
    border-radius: 14px 14px 4px 14px;
    background: var(--text-primary);
    color: var(--surface-panel);
  }
  .message-meta {
    display: flex;
    gap: 8px;
    font-size: 9px;
    color: var(--text-secondary);
    margin: 6px 2px 0;
  }
  .outbound .message-meta {
    justify-content: flex-end;
  }
  .composer {
    margin: 16px 24px 24px;
    border: 1px solid var(--border-default);
    border-radius: 10px;
    padding: 12px;
  }
  .composer textarea {
    border: 0;
    box-shadow: none;
    resize: vertical;
    min-height: 80px;
    background: transparent;
    margin: 0;
    padding: 4px;
  }
  .composer-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .composer-footer span {
    font-size: 10px;
    color: var(--text-secondary);
  }
  .thread-sidebar {
    padding: 24px;
    border-left: 1px solid var(--border-default);
  }
  .thread-sidebar h2 {
    font-size: 12px;
    margin: 0 0 20px;
  }
  .thread-sidebar dt {
    font-size: 10px;
    color: var(--text-secondary);
    margin-top: 18px;
  }
  .thread-sidebar dd {
    margin: 5px 0;
    font-size: 12px;
  }
  .thread-sidebar details {
    margin-top: 24px;
    font-size: 12px;
  }
  .thread-sidebar summary {
    cursor: pointer;
    margin-bottom: 16px;
  }
  .linked-contact {
    font-size: 13px;
  }
  .reply-unavailable {
    padding: 20px;
    color: var(--text-secondary);
    font-size: 12px;
  }
  .message-pagination {
    text-align: center;
    font-size: 12px;
  }
  @media (max-width: 900px) {
    .thread-layout {
      grid-template-columns: 1fr;
    }
    .thread-sidebar {
      border-left: 0;
      border-top: 1px solid var(--border-default);
    }
    .message-history {
      padding: 16px;
    }
    .composer {
      margin: 16px;
    }
    .message {
      max-width: 90%;
    }
  }
</style>
