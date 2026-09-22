<script lang="ts">
  let { data, form } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}/crm`);
</script>

<header>
  <a href={`/workspaces/${data.workspace.id}/contacts`}>← Contacts</a><span
    >{data.workspace.name}</span
  >
</header>
<h1>CRM</h1>
<p>
  Track potential business as opportunities, and live or past work as separate
  deals.
</p>
{#if form?.message}<p class="error" role="alert">{form.message}</p>{/if}
{#if data.module.enabled}
  <div class="columns">
    <section>
      <h2>Opportunities</h2>
      <p>Qualify interest and track each sales pursuit.</p>
      <a class="button" href={`${root}/opportunities`}>Open opportunities</a>
    </section>
    <section>
      <h2>Deals</h2>
      <p>
        Record transactions and engagements, with or without a prior
        opportunity.
      </p>
      <a class="button" href={`${root}/deals`}>Open deals</a>
    </section>
  </div>
  <section>
    <h2>Configuration</h2>
    <p>
      Define workflows for your business. Renaming a stage or status preserves
      its existing records.
    </p>
    <nav>
      <a href={`${root}/configuration/pipelines`}>Pipelines</a> ·
      <a href={`${root}/configuration/stages`}>Stages</a>
      · <a href={`${root}/configuration/categories`}>Categories</a> ·
      <a href={`${root}/configuration/statuses`}>Statuses</a>
    </nav>
  </section>
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
