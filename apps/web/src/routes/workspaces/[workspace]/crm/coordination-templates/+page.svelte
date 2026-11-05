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
<details class="creation-panel" open={Boolean(form?.message)}>
  <summary>Create a template</summary>
  <div class="creation-content">
    <form method="POST">
      <input
        type="hidden"
        name="idempotency_key"
        value={form?.draft?.key ?? data.key}
      /><CoordinationFields fields={form?.draft ?? data.fields} /><button
        >Create template</button
      >
    </form>
  </div>
</details>
