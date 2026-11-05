<script lang="ts">
  import AppearanceControls from '$lib/components/ui/AppearanceControls.svelte';
  import type { PageData } from './$types';
  let { data }: { data: PageData } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}`);
</script>

<svelte:head
  ><title>Settings · {data.workspace.name} · Looski</title></svelte:head
>
<h1>Settings</h1>
<div class="settings-grid">
  <section id="profile">
    <h2>Profile</h2>
    <dl>
      <dt>Email</dt>
      <dd>
        {data.accountEmail ?? 'Account details are temporarily unavailable.'}
      </dd>
      <dt>Workspace</dt>
      <dd>{data.workspace.name}</dd>
    </dl>
    <a href="/auth/password">Change password</a>
  </section>
  <section>
    <h2>Appearance</h2>
    <p>Saved for this browser.</p>
    <AppearanceControls />
  </section>
  <section>
    <h2>Workspace administration</h2>
    <nav class="settings-links">
      <a href={`${root}/access`}>Members &amp; permissions <span>→</span></a><a
        href={`${root}/tags`}>Tags <span>→</span></a
      ><a href={`${root}/crm`}>CRM configuration <span>→</span></a><a
        href={`${root}/exports`}>Data exports <span>→</span></a
      >
    </nav>
  </section>
  <section>
    <h2>Data &amp; customization</h2>
    <nav class="settings-links">
      <a href={`${root}/settings/fields?kind=contacts`}
        >Contact fields <span>→</span></a
      ><a href={`${root}/settings/fields?kind=organizations`}
        >Organization fields <span>→</span></a
      ><a href={`${root}/tags`}>Tags <span>→</span></a>
    </nav>
  </section>
  <section>
    <h2>Channels &amp; integrations</h2>
    <nav class="settings-links">
      <a href={`${root}/settings/connections`}
        >Connected accounts <span>→</span></a
      ><a href={`${root}/scheduling`}>Scheduling <span>→</span></a>
    </nav>
  </section>
</div>

<style>
  .settings-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 20px;
    max-width: 920px;
  }
  .settings-grid section {
    margin: 0;
  }
  dl {
    display: grid;
    gap: 8px;
    margin-bottom: 24px;
  }
  dt {
    font-size: 11px;
    color: var(--text-secondary);
    margin-top: 8px;
  }
  dd {
    margin: 0;
    font-size: 13px;
  }
  .settings-links {
    display: grid;
  }
  .settings-links a {
    display: flex;
    justify-content: space-between;
    padding: 8px 0;
    text-decoration: none;
    color: var(--text-primary);
    border-bottom: 1px solid var(--border-default);
  }
  @media (max-width: 700px) {
    .settings-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
