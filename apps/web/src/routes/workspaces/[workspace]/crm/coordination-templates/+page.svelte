<script lang="ts">
  import CoordinationFields from '$lib/components/CoordinationFields.svelte';
  let { data, form } = $props();
  const root = $derived(
    `/workspaces/${data.workspace.id}/crm/coordination-templates`,
  );
</script>

<header>
  <a href={`/workspaces/${data.workspace.id}/crm`}>← CRM</a><a
    href={`/workspaces/${data.workspace.id}/tasks/templates`}>Task templates</a
  >
</header>
<h1>Coordination templates</h1>
<p>
  Pinned task checklists. Applying a revision creates ordinary tasks; later
  edits preserve existing work.
</p>
{#if form?.message}<p role="alert" class="error">{form.message}</p>{/if}
<a href={`?archived=${!data.archived}`}
  >{data.archived ? 'Show active' : 'Show archived'}</a
>
<section>
  <h2>{data.archived ? 'Archived' : 'Active'} templates</h2>
  <ul>
    {#each data.page.items as template}<li>
        <a href={`${root}/${template.id}`}>{template.name}</a> · Revision {template
          .revision.number}
      </li>{:else}<li>No templates yet.</li>{/each}
  </ul>
  {#if data.page.next_cursor}<a
      href={`?archived=${data.archived}&after=${data.page.next_cursor}`}
      >More templates →</a
    >{/if}
</section>
<section>
  <h2>Create a template</h2>
  <form method="POST">
    <input
      type="hidden"
      name="idempotency_key"
      value={form?.draft?.key ?? data.key}
    /><CoordinationFields fields={form?.draft ?? data.fields} /><button
      >Create template</button
    >
  </form>
</section>
