<script lang="ts">
  import TaskTemplateFields from '$lib/components/TaskTemplateFields.svelte';
  let { data, form } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}/tasks/templates`);
</script>

<header>
  <a href={`/workspaces/${data.workspace.id}/tasks`}>← Tasks</a><a
    href={`/workspaces/${data.workspace.id}/tasks/types`}>Task types</a
  >
</header>
<h1>Task templates</h1>
<p>
  Reusable work with explicit assignment and due-date rules. Revisions preserve
  existing work.
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
    /><TaskTemplateFields fields={form?.draft ?? data.fields} /><button
      >Create template</button
    >
  </form>
</section>
