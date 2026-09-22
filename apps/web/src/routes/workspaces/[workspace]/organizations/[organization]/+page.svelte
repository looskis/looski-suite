<script lang="ts">
  import TagPanel from '$lib/components/TagPanel.svelte';
  import OrganizationFields from '$lib/components/OrganizationFields.svelte';
  import type { PageData, ActionData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
</script>

<svelte:head
  ><title>{data.organization.name} · Organizations · Looski</title></svelte:head
>
<header>
  <a href={`/workspaces/${data.workspace.id}/organizations`}>← Organizations</a
  ><span>{data.workspace.name}</span>
</header>
<h1>{data.organization.name}</h1>
<p class="muted">
  {data.organization.archived
    ? 'Archived · History is preserved.'
    : 'Active organization'}
</p>
{#if form?.message}<p role="alert" class="error">
    {form.message} <a href={``}>Reload current record</a>
  </p>{/if}
<section>
  <h2>Organization details</h2>
  <form method="POST" action="?/edit">
    <input
      type="hidden"
      name="idempotency_key"
      value={form?.idempotencyKey ?? data.editKey}
    />
    <input
      type="hidden"
      name="expected_version"
      value={form?.expectedVersion ?? data.organization.version}
    />
    <OrganizationFields
      organization={form?.input ?? data.organization}
    /><button>Save changes</button>
  </form>
</section>
<section>
  <h2>
    {data.organization.archived
      ? 'Restore organization'
      : 'Archive organization'}
  </h2>
  <p>
    Archiving keeps the organization and its history. You can restore it later.
  </p>
  <form method="POST" action="?/archive">
    <input type="hidden" name="idempotency_key" value={data.archiveKey} /><input
      type="hidden"
      name="expected_version"
      value={data.organization.version}
    /><input
      type="hidden"
      name="archived"
      value={String(!data.organization.archived)}
    /><button class="secondary"
      >{data.organization.archived
        ? 'Restore organization'
        : 'Archive organization'}</button
    >
  </form>
</section>

<TagPanel
  panel={data.tags}
  workspace={data.workspace.id}
  version={data.organization.version}
/>
