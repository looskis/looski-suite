<script lang="ts">
  let { data, form } = $props();
  const root = $derived(
    `/workspaces/${data.workspace.id}/crm/deals/${data.deal.id}`,
  );
</script>

<header>
  <a href={root}>← Deal</a><a
    href={`/workspaces/${data.workspace.id}/crm/coordination-templates`}
    >Coordination templates</a
  >
</header>
<h1>{data.deal.title}: coordination</h1>
{#if form?.message}<p role="alert" class="error">
    {form.message}
  </p>{/if}{#if form?.draft}<a href={`${root}/coordination`}
    >Discard draft and reload saved values</a
  >{/if}
<section>
  <h2>Applied checklists</h2>
  <ul>
    {#each data.page.items as instance}<li>
        <a href={`${root}/coordination/${instance.id}`}
          >{instance.template_name}</a
        >
        · Base date {instance.base_on} · {instance.created_at}
      </li>{:else}<li>No checklists applied.</li>{/each}
  </ul>
  {#if data.page.next_cursor}<a href={`?after=${data.page.next_cursor}`}
      >More checklists →</a
    >{/if}
</section>
<section>
  <h2>Apply a pinned revision</h2>
  <p>
    This creates the checklist and its tasks together. Existing work keeps its
    selected revision.
  </p>
  <form method="POST">
    <input
      type="hidden"
      name="idempotency_key"
      value={form?.draft?.key ?? data.key}
    /><input
      type="hidden"
      name="expected_version"
      value={form?.draft?.expected_version ?? data.deal.version}
    /><label
      >Coordination revision ID<input
        name="revision_id"
        value={form?.draft?.revision_id ?? ''}
        required
      /></label
    ><label
      >Base calendar date<input
        type="date"
        name="base_on"
        value={form?.draft?.base_on ?? data.today}
        required
      /></label
    ><button disabled={data.deal.archived}>Apply checklist</button>
  </form>
  <p class="muted">
    The default date is today in UTC; choose another date for your business
    calendar.
  </p>
</section>
