<script lang="ts">
  let { data, form } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}/crm`);
</script>

<svelte:head><title>CRM configuration · Looski</title></svelte:head>
<div class="page-heading">
  <h1>CRM configuration</h1>
  <span class="status-badge"
    >{data.module.enabled ? 'Enabled' : 'Disabled'}</span
  >
</div>
{#if form?.message}<p class="error" role="alert">{form.message}</p>{/if}
{#if data.module.enabled}
  <div class="configuration-grid">
    <section>
      <h2>Sales pipeline</h2>
      <nav>
        <a href={`${root}/opportunities`}>Opportunities <span>→</span></a><a
          href={`${root}/configuration/pipelines`}>Pipelines <span>→</span></a
        ><a href={`${root}/configuration/stages`}>Stages <span>→</span></a>
      </nav>
    </section>
    <section>
      <h2>Deals</h2>
      <nav>
        <a href={`${root}/deals`}>All deals <span>→</span></a><a
          href={`${root}/configuration/categories`}>Categories <span>→</span></a
        ><a href={`${root}/configuration/statuses`}>Statuses <span>→</span></a>
      </nav>
    </section>
    <section>
      <h2>Automation &amp; activities</h2>
      <nav>
        <a href={`${root}/coordination-rules`}>Status rules <span>→</span></a><a
          href={`${root}/coordination-templates`}
          >Coordination templates <span>→</span></a
        ><a href={`${root}/activity-types`}>Activity types <span>→</span></a>
      </nav>
    </section>
    <section>
      <h2>Setup</h2>
      <nav>
        <a href={`${root}/starter-packages`}>Starter packages <span>→</span></a
        ><a href={`/workspaces/${data.workspace.id}/projects/recipes`}
          >Deal-to-project recipes <span>→</span></a
        >
      </nav>
    </section>
  </div>
{/if}
<section>
  <h2>{data.module.enabled ? 'CRM is enabled' : 'Enable CRM'}</h2>
  <p>Disabling CRM preserves its data and keeps shared Contacts available.</p>
  <form method="POST">
    <input type="hidden" name="idempotency_key" value={data.key} /><input
      type="hidden"
      name="expected_version"
      value={data.module.version}
    /><input
      type="hidden"
      name="enabled"
      value={data.module.enabled ? 'false' : 'true'}
    /><button class="secondary"
      >{data.module.enabled ? 'Disable CRM' : 'Enable CRM'}</button
    >
  </form>
</section>

<style>
  .configuration-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 16px;
  }
  .configuration-grid section {
    margin: 0;
  }
  .configuration-grid nav {
    display: grid;
  }
  .configuration-grid a {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 0;
    border-bottom: 1px solid var(--border-default);
    color: var(--text-primary);
    text-decoration: none;
  }
  .configuration-grid a:last-child {
    border: 0;
  }
  @media (max-width: 700px) {
    .configuration-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
