<script lang="ts">
  import TagPanel from '$lib/components/TagPanel.svelte';
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

{#if data.canReadOrganizations}
  <section>
    <h2>Organizations</h2>
    <p>
      Keep multiple relationships and choose one primary organization. Changes
      retain earlier relationship details.
    </p>
    {#each data.relationships as relationship (relationship.id)}
      <form method="POST" action="?/relate">
        <h3>
          <a
            href={`/workspaces/${data.workspace.id}/organizations/${relationship.organization_id}`}
            >{relationship.organization_name}</a
          >
        </h3>
        <input
          type="hidden"
          name="organization_id"
          value={relationship.organization_id}
        /><input
          type="hidden"
          name="expected_contact_version"
          value={data.contact.version}
        /><input
          type="hidden"
          name="idempotency_key"
          value={data.relationshipKey}
        />
        <div class="columns">
          <label
            >Relationship<input
              name="relationship_label"
              maxlength="120"
              value={relationship.relationship_label}
            /></label
          ><label
            >Job title<input
              name="job_title"
              maxlength="120"
              value={relationship.job_title}
            /></label
          >
        </div>
        <label class="check"
          ><input
            type="checkbox"
            name="is_primary"
            checked={relationship.is_primary}
          /> Primary organization</label
        >
        <div class="toolbar">
          <button>Save relationship</button><button
            class="secondary"
            name="ended"
            value="true">End relationship</button
          >
        </div>
      </form>
    {:else}<p class="muted">No organization relationships yet.</p>{/each}
    <h3>Add a relationship</h3>
    <form method="GET" class="inline">
      <label
        >Find an organization<input
          name="organization_search"
          value={data.organizationSearch}
        /></label
      ><button>Find</button>
    </form>
    <form method="POST" action="?/relate">
      <input
        type="hidden"
        name="expected_contact_version"
        value={data.contact.version}
      /><input
        type="hidden"
        name="idempotency_key"
        value={data.relationshipKey}
      />
      <label
        >Organization<select name="organization_id" required
          ><option value="">Choose an organization</option
          >{#each data.organizations as organization (organization.id)}<option
              value={organization.id}>{organization.name}</option
            >{/each}</select
        ></label
      >
      <div class="columns">
        <label
          >Relationship<input
            name="relationship_label"
            maxlength="120"
            placeholder="e.g. Customer, Advisor"
          /></label
        ><label>Job title<input name="job_title" maxlength="120" /></label>
      </div>
      <label class="check"
        ><input type="checkbox" name="is_primary" /> Primary organization</label
      ><button>Add relationship</button>
      <a href={`/workspaces/${data.workspace.id}/organizations`}
        >Create an organization</a
      >
    </form>
  </section>
{/if}

<TagPanel
  draft={form?.tagDraft}
  panel={data.tags}
  workspace={data.workspace.id}
  version={data.contact.version}
/>
