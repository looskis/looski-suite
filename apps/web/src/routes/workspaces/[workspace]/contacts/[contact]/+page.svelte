<script lang="ts">
  import ContactFields from '$lib/components/ContactFields.svelte';
  import type { PageData, ActionData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
</script>

<svelte:head
  ><title>{data.contact.display_name} · Contacts · Looski</title></svelte:head
>
<header>
  <a href={`/workspaces/${data.workspace.id}/contacts`}>← Contacts</a><span
    >{data.workspace.name}</span
  >
</header>
<h1>{data.contact.display_name}</h1>
<p class="muted">
  {data.contact.archived
    ? 'Archived · History is preserved.'
    : 'Active contact'}
</p>
{#if form?.message}<p role="alert" class="error">
    {form.message} <a href={``}>Reload current record</a>
  </p>{/if}
<section>
  <h2>Contact details</h2>
  <form method="POST" action="?/edit">
    <input
      type="hidden"
      name="idempotency_key"
      value={form?.idempotencyKey ?? data.editKey}
    />
    <input
      type="hidden"
      name="expected_version"
      value={form?.expectedVersion ?? data.contact.version}
    />
    <ContactFields contact={form?.input ?? data.contact} /><button
      >Save changes</button
    >
  </form>
</section>
<section>
  <h2>{data.contact.archived ? 'Restore contact' : 'Archive contact'}</h2>
  <p>Archiving keeps the contact and its history. You can restore it later.</p>
  <form method="POST" action="?/archive">
    <input type="hidden" name="idempotency_key" value={data.archiveKey} /><input
      type="hidden"
      name="expected_version"
      value={data.contact.version}
    /><input
      type="hidden"
      name="archived"
      value={String(!data.contact.archived)}
    /><button class="secondary"
      >{data.contact.archived ? 'Restore contact' : 'Archive contact'}</button
    >
  </form>
</section>

<p>
  <a href={`/workspaces/${data.workspace.id}/contacts/${data.contact.id}/merge`}
    >Review a duplicate for merging →</a
  >
</p>
