<script lang="ts">
  import { shortDate } from '$lib/format';
  import type { PageData } from './$types';
  let { data }: { data: PageData } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}`);
</script>

<svelte:head><title>Inbox · {data.workspace.name} · Looski</title></svelte:head>
<div class="page-heading">
  <h1>Inbox</h1>
  <a class="button secondary" href={`${root}/settings/connections`}
    >Manage connections</a
  >
</div>
<nav class="view-tabs">
  <a href="?" aria-current={!data.archived ? 'page' : undefined}
    >Conversations</a
  ><a href="?archived=true" aria-current={data.archived ? 'page' : undefined}
    >Archived</a
  >
</nav>
{#if data.contact_id}<p class="muted">
    Filtered to one contact. <a href="?">Show all conversations</a>
  </p>{/if}
<form class="inbox-filters" method="GET" role="search">
  <input
    type="hidden"
    name="archived"
    value={String(data.archived)}
  />{#if data.contact_id}<input
      type="hidden"
      name="contact_id"
      value={data.contact_id}
    />{/if}<label class="sr-only" for="inbox-search">Search conversations</label
  ><input
    id="inbox-search"
    name="search"
    type="search"
    maxlength="200"
    value={data.search}
    placeholder="Search conversations…"
  /><label class="sr-only" for="inbox-account">Account</label><select
    id="inbox-account"
    name="account_id"
    value={data.accountId}
    ><option value="">All accounts</option
    >{#each data.accounts as account}<option value={account.id}
        >{account.name}</option
      >{/each}</select
  ><button class="secondary">Filter</button>
</form>
{#if data.conversations}<div class="inbox-list">
    {#each data.conversations.items as conversation}<a
        class="conversation-row"
        href={`${root}/communications/${conversation.id}`}
        ><span class="conversation-avatar" aria-hidden="true"
          >{(conversation.subject || conversation.channel)
            .slice(0, 2)
            .toUpperCase()}</span
        ><span class="conversation-copy"
          ><strong>{conversation.subject || 'Untitled conversation'}</strong
          ><span
            >{data.accounts.find((a) => a.id === conversation.account_id)
              ?.name ?? 'Connected account'}
            <span class="channel">{conversation.channel}</span></span
          ></span
        ><span class="conversation-meta"
          ><time datetime={conversation.updated_at}
            >{shortDate(conversation.updated_at)}</time
          ><span aria-hidden="true">↗</span></span
        ></a
      >{:else}<div class="empty-state">
        <span class="empty-icon" aria-hidden="true">↗</span><strong
          >{data.archived
            ? 'No archived conversations'
            : 'Your inbox is ready'}</strong
        >
        <p>
          {data.archived
            ? 'Archived conversations will appear here.'
            : 'Messages from your connected accounts will appear here.'}
        </p>
        {#if !data.archived}<a
            class="button secondary"
            href={`${root}/settings/connections`}>Connect an account</a
          >{/if}
      </div>{/each}
  </div>
  <div class="list-footer">
    <span>{data.conversations.items.length} conversations on this page</span
    >{#if data.conversations.next_cursor}<a
        href={`?${new URLSearchParams({ after: data.conversations.next_cursor, search: data.search, account_id: data.accountId, archived: String(data.archived), ...(data.contact_id ? { contact_id: data.contact_id } : {}) })}`}
        >Next page →</a
      >{/if}
  </div>
{:else}<div class="empty-state">
    <strong>Inbox is turned off</strong><a href={`${root}/settings/connections`}
      >Enable in Settings →</a
    >
  </div>{/if}

<style>
  .inbox-filters {
    display: flex;
    gap: 8px;
    align-items: center;
    margin-top: 20px;
  }
  .inbox-filters input {
    max-width: 420px;
    margin: 0;
  }
  .inbox-filters select {
    max-width: 220px;
    margin: 0;
  }
  @media (max-width: 600px) {
    .inbox-filters {
      flex-wrap: wrap;
    }
    .inbox-filters input {
      flex-basis: 100%;
    }
  }
  .inbox-list {
    border: 1px solid var(--border-default);
    border-radius: 10px;
    overflow: hidden;
    margin-top: 20px;
    background: var(--surface-panel);
  }
  .conversation-row {
    display: flex;
    gap: 16px;
    align-items: center;
    padding: 20px;
    text-decoration: none;
    color: var(--text-primary);
    border-bottom: 1px solid var(--border-default);
  }
  .conversation-row:last-child {
    border: 0;
  }
  .conversation-row:hover {
    background: var(--surface-hover);
  }
  .conversation-avatar {
    display: grid;
    place-items: center;
    width: 42px;
    height: 42px;
    flex-shrink: 0;
    border-radius: 14px;
    background: var(--surface-hover);
    border: 1px solid var(--border-default);
    font-size: 12px;
    font-weight: 600;
  }
  .conversation-copy {
    min-width: 0;
    display: grid;
    gap: 7px;
    flex: 1;
  }
  .conversation-copy strong {
    font-size: 14px;
    font-weight: 550;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .conversation-copy > span,
  .conversation-meta {
    font-size: 11px;
    color: var(--text-secondary);
  }
  .channel {
    margin-left: 8px;
    padding: 2px 6px;
    border: 1px solid var(--border-default);
    border-radius: 4px;
  }
  .conversation-meta {
    display: grid;
    gap: 12px;
    text-align: right;
  }
  .empty-icon {
    font-size: 30px;
    display: block;
    margin-bottom: 20px;
  }
  .empty-state {
    padding: 72px 20px;
  }
</style>
