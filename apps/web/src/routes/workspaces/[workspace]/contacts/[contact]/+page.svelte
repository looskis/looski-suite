<script lang="ts">
  import RecordProperties from '$lib/components/RecordProperties.svelte';
  import RecordDrawer from '$lib/components/ui/RecordDrawer.svelte';
  import { shortDate } from '$lib/format';
  import TagPanel from '$lib/components/TagPanel.svelte';
  import ContactFields from '$lib/components/ContactFields.svelte';
  import type { PageData, ActionData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
</script>

<svelte:head
  ><title>{data.contact.display_name} · Contacts · Looski</title></svelte:head
>
<header>
  <a href={`/workspaces/${data.workspace.id}/contacts`}>← Contacts</a>
</header>

<div class="page-heading">
  <h1>
    {data.contact.display_name}<span class="status-badge"
      >{data.contact.archived ? 'Archived' : 'Active'}</span
    >
  </h1>
  <div class="page-actions">
    <a
      class="button secondary"
      href={`/workspaces/${data.workspace.id}/contacts/${data.contact.id}/merge`}
      >Merge duplicate</a
    >
    <RecordDrawer
      message={form?.message}
      title="Edit contact"
      label="Edit contact"
      secondary
      error={Boolean(form?.input)}
    >
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
        <ContactFields
          fields={data.fields}
          contact={form?.input ?? data.contact}
        /><button>Save changes</button>
      </form>
    </RecordDrawer>
  </div>
</div>
<nav class="view-tabs" aria-label="Contact">
  <a href={`?`} aria-current="page">Overview</a><a
    href={`/workspaces/${data.workspace.id}/crm/timeline?anchor_kind=contact&anchor_id=${data.contact.id}`}
    >Timeline</a
  ><a
    href={`/workspaces/${data.workspace.id}/crm/activities?anchor_kind=contact&anchor_id=${data.contact.id}`}
    >Activities</a
  >
</nav>
<dl class="record-summary">
  <div>
    <dt>Email</dt>
    <dd>
      {data.contact.identifiers
        .filter((i) => i.kind === 'email')
        .map((i) => i.value)
        .join(', ') || '—'}
    </dd>
  </div>
  <div>
    <dt>Phone</dt>
    <dd>
      {data.contact.identifiers
        .filter((i) => i.kind === 'phone')
        .map((i) => i.value)
        .join(', ') || '—'}
    </dd>
  </div>
  <div>
    <dt>Created</dt>
    <dd>{shortDate(data.contact.created_at)}</dd>
  </div>
  <div>
    <dt>Updated</dt>
    <dd>{shortDate(data.contact.updated_at)}</dd>
  </div>
</dl>
<div class="record-layout">
  <div class="record-main">
    <section>
      <h2>Notes</h2>
      <p class="record-notes">{data.contact.notes || 'No notes added.'}</p>
    </section>

    {#if form?.message}<p role="alert" class="error">
        {form.message} <a href={``}>Reload current record</a>
      </p>{/if}

    {#if data.canReadOrganizations}
      <section>
        <h2>Organizations</h2>
        {#each data.relationships as relationship (relationship.id)}
          <details class="relationship-editor">
            <summary
              >{relationship.organization_name}
              <span class="muted"
                >{relationship.job_title ||
                  relationship.relationship_label}{relationship.is_primary
                  ? ' · Primary'
                  : ''}</span
              ></summary
            >
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
          </details>
        {:else}<p class="muted">No organization relationships yet.</p>{/each}
        <details open={Boolean(data.organizationSearch)}>
          <summary>Add an organization</summary>
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
              ><label>Job title<input name="job_title" maxlength="120" /></label
              >
            </div>
            <label class="check"
              ><input type="checkbox" name="is_primary" /> Primary organization</label
            ><button>Add relationship</button>
            <a href={`/workspaces/${data.workspace.id}/organizations`}
              >Create an organization</a
            >
          </form>
        </details>
      </section>
    {/if}

    {#if data.contact.archived}<p>
        <a
          href={`/workspaces/${data.workspace.id}/contacts/${data.contact.id}/delete`}
          >Review permanent deletion</a
        >
      </p>{/if}

    {#if data.conversations}<section>
        <h2>Conversations</h2>
        {#each data.conversations.items as c}<p>
            <a href={`/workspaces/${data.workspace.id}/communications/${c.id}`}
              >{c.subject || c.provider_thread_id}</a
            >
            · {c.channel} · {c.updated_at}
          </p>{:else}<p>No linked conversations yet.</p>{/each}
        <a
          href={`/workspaces/${data.workspace.id}/communications?contact_id=${data.contact.id}`}
          >All conversations →</a
        >
      </section>{/if}
  </div>
  <aside class="record-side">
    <RecordProperties
      fields={data.fields}
      properties={data.contact.properties}
    />
    <TagPanel
      draft={form?.tagDraft}
      panel={data.tags}
      workspace={data.workspace.id}
      version={data.contact.version}
    />
    <section>
      <h2>{data.contact.archived ? 'Restore contact' : 'Archive contact'}</h2>
      <p>
        Archiving keeps the contact and its history. You can restore it later.
      </p>
      <form method="POST" action="?/archive">
        <input
          type="hidden"
          name="idempotency_key"
          value={data.archiveKey}
        /><input
          type="hidden"
          name="expected_version"
          value={data.contact.version}
        /><input
          type="hidden"
          name="archived"
          value={String(!data.contact.archived)}
        /><button class="secondary"
          >{data.contact.archived
            ? 'Restore contact'
            : 'Archive contact'}</button
        >
      </form>
    </section>
  </aside>
</div>

<style>
  .record-notes {
    white-space: pre-wrap;
  }
  .record-main > section:first-child {
    margin-top: 0;
  }
  .record-side :global(section:first-child) {
    margin-top: 0;
  }
</style>
