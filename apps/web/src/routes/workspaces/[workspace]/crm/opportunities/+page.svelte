<script lang="ts">
  import CrmRecordFields from '$lib/components/CrmRecordFields.svelte';
  import CrmWorkflowPicker from '$lib/components/CrmWorkflowPicker.svelte';
  let { data, form } = $props();
  let selectedOrigin = $state('');
  $effect(() => {
    selectedOrigin = form?.draft?.origin ?? data.origin;
  });
  const workflow = $derived({
    ...data.workflow,
    parent: form?.draft?.parent_id ?? data.workflow.parent,
    state: form?.draft?.state_id ?? data.workflow.state,
  });
</script>

<header>
  <a href={`/workspaces/${data.workspace.id}/crm`}>← CRM</a><a
    href={`?archived=${!data.archived}`}
    >{data.archived ? 'View active' : 'View archived'}</a
  >
</header>
<h1>Opportunities</h1>
{#if form?.message}<p class="error" role="alert">{form.message}</p>{/if}
<form method="GET" class="inline">
  <label>Search<input name="search" value={data.search} /></label><button
    class="secondary">Search</button
  >
</form>
<section>
  <div class="table-wrap">
    <table>
      <thead><tr><th>Title</th><th>Stage</th><th>Value</th></tr></thead><tbody
        >{#each data.page.items as record}<tr
            ><td
              ><a
                href={`/workspaces/${data.workspace.id}/crm/opportunities/${record.id}`}
                >{record.title}</a
              ></td
            ><td>{record.stage_name}</td><td
              >{record.expected_value ?? '—'} {record.currency_code ?? ''}</td
            ></tr
          >{/each}</tbody
      >
    </table>
  </div>
  {#if data.page.next_cursor}<p>
      <a
        href={`?after=${data.page.next_cursor}&search=${encodeURIComponent(data.search)}&archived=${data.archived}`}
        >More opportunities →</a
      >
    </p>{/if}
</section>
<section>
  <h2>Create opportunity</h2>
  <CrmWorkflowPicker isDeal={false} {workflow} origin={selectedOrigin} />
  {#if data.workflow.states.length}<form method="POST">
      <input
        type="hidden"
        name="idempotency_key"
        value={form?.draft?.key ?? data.key}
      /><CrmRecordFields
        isDeal={false}
        fields={form?.draft ?? data.fields}
        {workflow}
        origins={data.origins}
        bind:origin={selectedOrigin}
        creating
      /><button>Create opportunity</button>
    </form>{:else}<p>
      Create or select a workflow with an active stage in <a
        href={`/workspaces/${data.workspace.id}/crm/configuration/pipelines`}
        >Configuration</a
      >.
    </p>{/if}
</section>
