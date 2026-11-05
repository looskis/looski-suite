<script lang="ts">
  import MoneyValue from '$lib/components/MoneyValue.svelte';
  import { shortDate } from '$lib/format';
  import RecordDrawer from '$lib/components/ui/RecordDrawer.svelte';
  import { page } from '$app/state';
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

<div class="page-heading">
  <h1>
    Deals<span class="count"
      >{data.page.items.length}{data.page.next_cursor ? '+' : ''} shown</span
    >
  </h1>
  <RecordDrawer
    message={form?.message}
    title="New deal"
    error={Boolean(form?.message) || page.url.searchParams.has('parent')}
  >
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
  </RecordDrawer>
</div>
<nav class="view-tabs" aria-label="Deals views">
  <a href="?" aria-current={!data.archived ? 'page' : undefined}>All deals</a><a
    href="?archived=true"
    aria-current={data.archived ? 'page' : undefined}>Archived</a
  >
</nav>
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

<section class="index-list">
  <div class="record-table">
    <table>
      <thead
        ><tr
          ><th>Title</th><th>Category</th><th>Status</th><th>Value</th><th
            >End date</th
          ><th>Updated</th></tr
        ></thead
      ><tbody
        >{#each data.page.items as record}<tr
            ><td
              ><a
                href={`/workspaces/${data.workspace.id}/crm/deals/${record.id}`}
                >{record.title}</a
              ></td
            ><td>{record.category_name}</td><td
              ><span class="status-badge">{record.status_name}</span></td
            ><td
              ><MoneyValue
                amount={record.value}
                currency={record.currency_code}
                billingFrequency={record.billing_frequency}
              /></td
            ><td class="cell-date">{record.ends_on ?? '—'}</td><td
              class="cell-date">{shortDate(record.updated_at)}</td
            ></tr
          >{:else}<tr><td colspan="6">No deals match these filters.</td></tr
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
