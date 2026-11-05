<script lang="ts">
  import type { PageData, ActionData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const root = $derived(
    `/workspaces/${data.workspace.id}/settings/connections`,
  );
</script>

<svelte:head><title>Connections · Settings · Looski</title></svelte:head><a
  class="back-link"
  href={`/workspaces/${data.workspace.id}/settings`}>← Settings</a
>
<div class="page-heading">
  <h1>Connections</h1>
  <a href={`/workspaces/${data.workspace.id}/communications`}>Open inbox →</a>
</div>
{#if form?.message}<p class="error" role="alert">{form.message}</p>{/if}
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
          ><input type="checkbox" name="enabled" checked={a.enabled} /> Receive and
          send messages</label
        >
        <label
          ><input type="checkbox" name="shared" checked={a.shared} /> Share with staff
          who have Communications access</label
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
<details class="creation-panel" open={Boolean(form?.message)}>
  <summary>Connect an account</summary>
  <div class="creation-content">
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
  </div>
</details>
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
            ? 'Turn off Communications'
            : 'Turn on Communications'}</button
        >
      </form>
    </div>
  </details>{/if}

<style>
  article {
    padding: 1rem 0;
    border-bottom: 1px solid var(--border-subtle);
  }
  article form + form {
    margin-top: 0.75rem;
  }
</style>
