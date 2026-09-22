<script lang="ts">
  import CrmRecordFields from '$lib/components/CrmRecordFields.svelte';
  import CrmWorkflowPicker from '$lib/components/CrmWorkflowPicker.svelte';
  let { data, form } = $props();
  let selectedOrigin = $derived(form?.draft?.origin ?? data.origin);
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
<h1>Deals</h1>
{#if form?.message}<p class="error" role="alert">{form.message}</p>{/if}
<form method="GET" class="inline">
  <input
    type="hidden"
    name="archived"
    value={data.archived ? 'true' : 'false'}
  />
  <label>Search<input name="search" value={data.search} /></label>
  <label
    >Tag<select name="tag_id" value={data.tagId}
      ><option value="">Any tag</option>
      {#if data.tagId && !data.tagChoices.some((t) => t.id === data.tagId)}<option
          value={data.tagId}>Selected tag</option
        >{/if}
      {#each data.tagChoices as tag}<option value={tag.id}>{tag.name}</option
        >{/each}</select
    ></label
  ><button class="secondary">Search</button>
</form>
<p>
  <a href={`/workspaces/${data.workspace.id}/tags`}
    >Find any tag in the full catalog</a
  >
</p>
<section>
  <div class="table-wrap">
    <table>
      <thead><tr><th>Title</th><th>Status</th><th>Value</th></tr></thead><tbody
        >{#each data.page.items as record}<tr
            ><td
              ><a
                href={`/workspaces/${data.workspace.id}/crm/deals/${record.id}`}
                >{record.title}</a
              ></td
            ><td>{record.status_name}</td><td
              >{record.value ?? '—'} {record.currency_code ?? ''}</td
            ></tr
          >{/each}</tbody
      >
    </table>
  </div>
  {#if data.page.next_cursor}<p>
      <a
        href={`?after=${data.page.next_cursor}&search=${encodeURIComponent(data.search)}&archived=${data.archived}&tag_id=${data.tagId}`}
        >More deals →</a
      >
    </p>{/if}
</section>
<section>
  <h2>Create deal</h2>
  <CrmWorkflowPicker isDeal={true} {workflow} origin={selectedOrigin} />
  {#if data.workflow.states.length}<form method="POST">
      <input
        type="hidden"
        name="idempotency_key"
        value={form?.draft?.key ?? data.key}
      /><CrmRecordFields
        isDeal={true}
        fields={form?.draft ?? data.fields}
        {workflow}
        origins={data.origins}
        bind:origin={selectedOrigin}
        creating
      /><button>Create deal</button>
    </form>{:else}<p>
      Create or select a workflow with an active status in <a
        href={`/workspaces/${data.workspace.id}/crm/configuration/categories`}
        >Configuration</a
      >.
    </p>{/if}
</section>
