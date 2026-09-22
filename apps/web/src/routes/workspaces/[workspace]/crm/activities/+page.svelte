<script lang="ts">
  import ActivityFields from '$lib/components/ActivityFields.svelte';
  let { data, form } = $props();
  const query = $derived(
    `anchor_kind=${data.kind}&anchor_id=${data.id}&type_id=${encodeURIComponent(data.typeFilter)}`,
  );
  const root = $derived(`/workspaces/${data.workspace.id}/crm`);
</script>

<header>
  <a href={root}>← CRM</a><a href={`${root}/activity-types`}
    >Configure activity types</a
  >
</header>
<h1>Activities</h1>
<form method="GET">
  <input type="hidden" name="anchor_kind" value={data.kind} /><input
    type="hidden"
    name="anchor_id"
    value={data.id}
  /><input type="hidden" name="archived" value={String(data.archived)} /><label
    >Filter by activity type ID<input
      name="type_id"
      value={data.typeFilter}
      list="activity-types"
    /></label
  ><button class="secondary">Filter</button>
</form>
<p>Notes and interactions for this {data.kind}.</p>
<a href={`?${query}&archived=${!data.archived}`}
  >{data.archived ? 'Show active' : 'Show archived'}</a
>
{#if form?.message}<p class="error" role="alert">{form.message}</p>{/if}
{#if form?.draft}<a href={`?${query}`}>Discard draft and reload saved values</a
  >{/if}
<section>
  <h2>{data.archived ? 'Archived' : 'Recorded'} activities</h2>
  {#each data.page.items as activity}<article>
      <h3>
        <a href={`${root}/activities/${activity.id}`}>{activity.subject}</a>
      </h3>
      <p>{activity.type_name} · {activity.occurred_at}</p>
      <p>{activity.body}</p>
    </article>{:else}<p>No visible activities.</p>{/each}
  {#if data.page.next_cursor}<a
      href={`?${query}&archived=${data.archived}&after=${data.page.next_cursor}`}
      >More activities →</a
    >{/if}
</section>
<section>
  <h2>Record an activity</h2>
  <form method="POST">
    <input
      type="hidden"
      name="anchor_kind"
      value={form?.draft?.anchor_kind ?? data.kind}
    /><input
      type="hidden"
      name="anchor_id"
      value={form?.draft?.anchor_id ?? data.id}
    />
    <input
      type="hidden"
      name="idempotency_key"
      value={form?.draft?.key ?? data.key}
    /><ActivityFields
      fields={form?.draft ?? data.fields}
      types={data.types}
    /><button>Record activity</button>
  </form>
</section>
