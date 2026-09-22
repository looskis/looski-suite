<script lang="ts">
  let { data, form } = $props();
  const root = $derived(
    `/workspaces/${data.workspace.id}/crm/starter-packages`,
  );
</script>

<header><a href={`/workspaces/${data.workspace.id}/crm`}>← CRM</a></header>
<h1>Starter package</h1>
<h2>{data.page.package.name} · version {data.page.package.version}</h2>
<p>{data.page.package.description}</p>
<p>
  Install an editable starting point: Sales stages (New, Qualified, Proposal,
  Won, Lost), General deal statuses (Planned, Active, Completed), Note and
  Interaction activity types, and a Follow-up task type. A transition to Active
  creates a Review requirements task from the Start delivery checklist.
</p>
<p>
  One installation can be active at a time. Removal archives unchanged defaults
  and retains customized definitions and active configuration dependencies.
  Business records, tasks and history remain available. Reinstallation creates
  new definitions.
</p>
{#if form?.message}<p role="alert" class="error">{form.message}</p>{/if}
<form method="POST">
  <input
    type="hidden"
    name="idempotency_key"
    value={form?.draft?.key ?? data.key}
  /><input
    type="hidden"
    name="package_version"
    value={form?.draft?.package_version ?? data.page.package.version}
  /><button>Install General CRM</button>
</form>
<section>
  <h2>Installations</h2>
  <ul>
    {#each data.page.items as item}<li>
        <a href={`${root}/${item.id}`}
          >{item.installed_at} · version {item.package_version} · {item.removed
            ? 'Removed'
            : 'Active'}</a
        >
      </li>{:else}<li>No installations yet.</li>{/each}
  </ul>
  {#if data.page.next_cursor}<a href={`?after=${data.page.next_cursor}`}
      >More installations →</a
    >{/if}
</section>
