<script lang="ts">
  import CoordinationFields from '$lib/components/CoordinationFields.svelte';
  let { data, form } = $props();
  const root = $derived(
    `/workspaces/${data.workspace.id}/crm/coordination-templates`,
  );
</script>

<header>
  <a href={root}>← Coordination templates</a><a
    href={`/workspaces/${data.workspace.id}/tasks/templates`}>Task templates</a
  >
</header>
<h1>{data.template.name}</h1>
<p>
  Revision {data.template.revision.number}{data.template.archived
    ? ' · archived'
    : ''}
</p>
{#if form?.message}<p role="alert" class="error">
    {form.message}
  </p>{/if}{#if form?.draft}<a href={`${root}/${data.template.id}`}
    >Discard draft and reload saved values</a
  >{/if}
<section>
  <h2>Template definition</h2>
  <form method="POST">
    <input
      type="hidden"
      name="idempotency_key"
      value={form?.draft?.key ?? data.key}
    /><input
      type="hidden"
      name="expected_version"
      value={form?.draft?.expected_version ?? data.template.version}
    /><CoordinationFields fields={form?.draft ?? data.fields} /><button
      >Save template</button
    >
  </form>
</section>
<section>
  <h2>Immutable revisions</h2>
  {#each data.revisions.items as revision}<article>
      <h3>{revision.number}: {revision.name}</h3>
      <p>Revision ID: {revision.id} · {revision.created_at}</p>
      <ol>
        {#each revision.items as item}<li>
            {item.label} · Task revision {item.task_template_revision_id}
          </li>{/each}
      </ol>
    </article>{/each}{#if data.revisions.next_cursor}<a
      href={`?after=${data.revisions.next_cursor}`}>More revisions →</a
    >{/if}
</section>
