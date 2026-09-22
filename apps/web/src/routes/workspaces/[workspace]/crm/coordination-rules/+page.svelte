<script lang="ts">
  import RuleFields from '$lib/components/RuleFields.svelte';
  let { data, form } = $props();
  const root = $derived(
    `/workspaces/${data.workspace.id}/crm/coordination-rules`,
  );
</script>

<header>
  <a href={`/workspaces/${data.workspace.id}/crm`}>← CRM</a><a
    href={`/workspaces/${data.workspace.id}/crm/coordination-templates`}
    >Checklist templates</a
  ><a href={`/workspaces/${data.workspace.id}/jobs`}>Background work</a>
</header>
<h1>Status rules</h1>
<p>
  A transition into the selected status queues a pinned checklist. Creating a
  deal, editing its other fields or renaming a status does not trigger work.
  Calendar offsets use the transition date in UTC.
</p>
{#if form?.message}<p class="error" role="alert">{form.message}</p>{/if}
<section>
  <h2>{data.archived ? 'Archived' : 'Active'} rules</h2>
  <a href={`?archived=${!data.archived}`}
    >Show {data.archived ? 'active' : 'archived'}</a
  >
  <ul>
    {#each data.page.items as rule}<li>
        <a href={`${root}/${rule.id}`}>{rule.name}</a>
      </li>{:else}<li>No rules yet.</li>{/each}
  </ul>
  {#if data.page.next_cursor}<a
      href={`?archived=${data.archived}&after=${data.page.next_cursor}`}
      >More rules →</a
    >{/if}
</section>
<section>
  <h2>Create rule</h2>
  <p>
    One active rule per status. Rule edits affect future transitions; already
    scheduled work retains its selected revision.
  </p>
  <form method="POST">
    <input
      type="hidden"
      name="idempotency_key"
      value={form?.draft?.key ?? data.key}
    /><RuleFields fields={form?.draft ?? data.fields} /><button
      >Create rule</button
    >
  </form>
</section>
