<script lang="ts">
  import ActivityFields from '$lib/components/ActivityFields.svelte';
  let { data, form } = $props();
</script>

<header>
  <a
    href={`/workspaces/${data.workspace.id}/crm/activities?anchor_kind=${data.activity.anchor_kind}&anchor_id=${data.activity.anchor_id}`}
    >← Activities</a
  ><span>{data.activity.archived ? 'Archived' : data.activity.type_name}</span>
</header>
<h1>{data.activity.subject}</h1>
{#if form?.message}<p role="alert" class="error">{form.message}</p>{/if}
{#if form?.draft || form?.archiveDraft}<p>
    <a
      href={`/workspaces/${data.workspace.id}/crm/activities/${data.activity.id}`}
      >Discard draft and reload saved values</a
    >
  </p>{/if}
<section>
  <h2>Activity details</h2>
  <form method="POST" action="?/save">
    <input
      type="hidden"
      name="idempotency_key"
      value={form?.draft?.key ?? data.key}
    />
    <input
      type="hidden"
      name="expected_version"
      value={form?.draft?.expected_version ?? data.activity.version}
    />
    <ActivityFields
      fields={form?.draft ?? data.fields}
      types={data.types}
    /><button disabled={data.activity.archived}>Save activity</button>
  </form>
  {#if data.activity.archived}<p>Restore this activity before editing.</p>{/if}
  <form method="POST" action="?/archive">
    <input
      type="hidden"
      name="idempotency_key"
      value={form?.archiveDraft?.key ?? data.archiveKey}
    />
    <input
      type="hidden"
      name="expected_version"
      value={form?.archiveDraft?.expected_version ?? data.activity.version}
    />
    <input
      type="hidden"
      name="archived"
      value={form?.archiveDraft?.archived ?? String(!data.activity.archived)}
    />
    <button class="secondary"
      >{data.activity.archived
        ? 'Restore activity'
        : 'Archive activity'}</button
    >
  </form>
</section>
<section>
  <h2>History</h2>
  <ol>
    {#each data.history.items as change}<li>
        <strong>{change.snapshot.subject}</strong> — {change.snapshot.type_name} ({change
          .snapshot.behavior})
        <p>
          Version {change.snapshot.version} · occurred {change.snapshot
            .occurred_at} · saved {change.occurred_at}{change.snapshot.archived
            ? ' · archived'
            : ''}
        </p>
        <p>{change.snapshot.body}</p>
        <p>
          Responsible: {change.snapshot.responsible_membership_id} · Duration: {change
            .snapshot.duration_minutes ?? 'Not recorded'}
        </p>
      </li>{/each}
  </ol>
  {#if data.history.next_cursor}<a
      href={`?history_after=${data.history.next_cursor}`}>More history →</a
    >{/if}
</section>
