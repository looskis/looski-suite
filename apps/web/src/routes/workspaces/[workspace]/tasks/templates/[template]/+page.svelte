<script lang="ts">
  import TaskTemplateFields from '$lib/components/TaskTemplateFields.svelte';
  let { data, form } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}/tasks/templates`);
</script>

<header><a href={root}>← Task templates</a></header>
<h1>{data.template.name}</h1>
<p>
  Revision {data.template.revision.number}{data.template.archived
    ? ' · archived'
    : ''}
</p>
{#if form?.message}<p role="alert" class="error">
    {form.message}
  </p>{/if}{#if form?.draft || form?.instantiateDraft}<a
    href={`${root}/${data.template.id}`}
    >Discard draft and reload saved values</a
  >{/if}
<section>
  <h2>Template definition</h2>
  <p>
    Changing the work specification creates a new revision. Existing tasks keep
    their original work.
  </p>
  <form method="POST" action="?/save">
    <input
      type="hidden"
      name="idempotency_key"
      value={form?.draft?.key ?? data.key}
    /><input
      type="hidden"
      name="expected_version"
      value={form?.draft?.expected_version ?? data.template.version}
    /><TaskTemplateFields fields={form?.draft ?? data.fields} /><button
      >Save template</button
    >
  </form>
</section>
<section>
  <h2>Create a task from a revision</h2>
  <form method="POST" action="?/instantiate">
    <input
      type="hidden"
      name="idempotency_key"
      value={form?.instantiateDraft?.key ?? data.instantiateKey}
    /><label
      >Revision ID<input
        name="revision_id"
        value={form?.instantiateDraft?.revision_id ?? data.template.revision.id}
        required
      /></label
    ><label
      >Base calendar date<input
        type="date"
        name="base_on"
        value={form?.instantiateDraft?.base_on ?? data.today}
        required
      /></label
    ><label
      >Contextual owner membership ID (when required)<input
        name="context_owner_membership_id"
        value={form?.instantiateDraft?.context_owner_membership_id ?? ''}
      /></label
    ><button disabled={data.template.archived}>Create task</button>
  </form>
  <p class="muted">
    The default base date is today in UTC. Choose another date if your business
    day differs.
  </p>
</section>
<section>
  <h2>Immutable revisions</h2>
  <ol>
    {#each data.revisions.items as revision}<li>
        <strong>{revision.number}: {revision.title}</strong>
        <p>{revision.notes}</p>
        <p>
          Assignment: {revision.assignment} · Offset: {revision.due_offset_days} calendar
          days · Type: {revision.type_name ?? 'Unspecified'}
        </p>
        <p>Revision ID: {revision.id} · {revision.created_at}</p>
      </li>{/each}
  </ol>
  {#if data.revisions.next_cursor}<a
      href={`?after=${data.revisions.next_cursor}`}>More revisions →</a
    >{/if}
</section>
