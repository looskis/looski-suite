<script lang="ts">
  import type { PageData, ActionData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}/communications`);
</script>

<svelte:head><title>Communications · Looski</title></svelte:head>
<header>
  <h1>Communications</h1>
  <span>{data.workspace.name}</span>
</header>
<p>Customer conversations across your connected accounts.</p>
{#if form?.message}<p role="alert" class="error">
    {form.message} <a href={root}>Reload</a>
  </p>{/if}
{#if data.conversations}
  <section>
    <div class="section-heading">
      <h2>{data.archived ? 'Archived conversations' : 'Inbox'}</h2>
      <a href={`?archived=${!data.archived}`}
        >{data.archived ? 'Show active' : 'Show archived'}</a
      >
    </div>
    {#if data.contact_id}<p>
        Showing conversations linked to this contact. <a href={root}>Show all</a
        >
      </p>{/if}
    <div class="table-wrap">
      <table>
        <thead
          ><tr
            ><th>Conversation</th><th>Channel</th><th>Account</th><th
              >Last activity</th
            ></tr
          ></thead
        ><tbody>
          {#each data.conversations.items as c}<tr
              ><td
                ><a href={`${root}/${c.id}`}
                  >{c.subject || c.provider_thread_id}</a
                ></td
              ><td>{c.channel}</td><td
                >{data.accounts.find((a) => a.id === c.account_id)?.name ??
                  'Connected account'}</td
              ><td>{c.updated_at}</td></tr
            >
          {:else}<tr
              ><td colspan="4"
                >No conversations yet. Connect an account below; incoming
                messages will appear here.</td
              ></tr
            >{/each}
        </tbody>
      </table>
    </div>
    {#if data.conversations.next_cursor}<a
        href={`?${new URLSearchParams({ after: data.conversations.next_cursor, archived: String(data.archived), ...(data.contact_id ? { contact_id: data.contact_id } : {}) })}`}
        >Next page →</a
      >{/if}
  </section>
  <section>
    <h2>Connected accounts</h2>
    {#each data.accounts as a}<article>
        <h3>{a.name}</h3>
        <p>
          {a.capabilities.channels.join(', ')} · {a.shared
            ? 'Shared with authorized staff'
            : 'Private to owner'} · {a.enabled ? 'Active' : 'Paused'}
        </p>
        {#if a.sync_error}<p role="alert">
            Connection needs attention. Check the connector and refresh.
          </p>{/if}
        <form method="POST" action="?/account">
          <input type="hidden" name="id" value={a.id} /><input
            type="hidden"
            name="idempotency_key"
            value={data.key + a.id}
          /><input type="hidden" name="expected_version" value={a.version} />
          <label
            >Name<input
              name="name"
              value={a.name}
              required
              maxlength="120"
            /></label
          >
          <label
            ><input type="checkbox" name="enabled" checked={a.enabled} /> Receive
            and send messages</label
          >
          <label
            ><input type="checkbox" name="shared" checked={a.shared} /> Share with
            staff who have Communications access</label
          >
          <button class="secondary">Save connection</button>
        </form>
        <form method="POST" action="?/sync">
          <input type="hidden" name="id" value={a.id} /><button
            class="secondary"
            disabled={!a.enabled}>Refresh messages</button
          >
        </form>
      </article>{/each}
  </section>
  <section>
    <h2>Connect an account</h2>
    <p>
      Your administrator supplies a connection key after configuring Blueski or
      another connector. New connections are private unless you share them.
    </p>
    <form method="POST" action="?/connect">
      <input
        type="hidden"
        name="idempotency_key"
        value={form?.key ?? data.key}
      />
      <label
        >Name<input
          name="name"
          value={form?.input?.name ?? ''}
          required
          maxlength="120"
          placeholder="Customer messages"
        /></label
      >
      <label
        >Connection key<input
          name="connector_key"
          value={form?.input?.connector_key ?? ''}
          required
          maxlength="120"
        /></label
      >
      <label
        ><input
          type="checkbox"
          name="shared"
          checked={form?.input?.shared ?? false}
        /> Share with authorized staff</label
      >
      <button>Connect account</button>
    </form>
  </section>
{:else}<section>
    <h2>Communications is turned off</h2>
    <p>Enable it to connect accounts and manage conversations.</p>
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
          ? 'Turn off Communications'
          : 'Turn on Communications'}</button
      >
    </form>
  </section>{/if}

<style>
  article {
    padding: 1rem 0;
    border-bottom: 1px solid var(--border, #ddd);
  }
  article form + form {
    margin-top: 0.75rem;
  }
</style>
