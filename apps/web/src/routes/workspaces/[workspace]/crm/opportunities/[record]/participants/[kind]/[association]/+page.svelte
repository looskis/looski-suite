<script lang="ts">
  let { data, form } = $props();
</script>

<header>
  <a
    href={`/workspaces/${data.workspace.id}/crm/${data.sales}/${data.record.id}/participants/${data.kind}`}
    >← Participants</a
  ><span>{data.record.title}</span>
</header>
<h1>{data.participant.name}</h1>
<p>
  {data.participant.ended ? 'Ended' : 'Active'} association{data.participant
    .is_primary
    ? ' · Primary'
    : ''}
</p>
<a
  href={`/workspaces/${data.workspace.id}/${data.kind}/${data.participant.canonical_id}`}
  >View {data.kind === 'contacts' ? 'contact' : 'organization'}</a
>
{#if data.participant.reference_id !== data.participant.canonical_id}<p>
    The original contact was merged. This association keeps its original
    reference and history.
  </p>{/if}
{#if form?.message}<p class="error" role="alert">{form.message}</p>{/if}
<section>
  <h2>Edit association</h2>
  <form method="POST">
    <input
      type="hidden"
      name="idempotency_key"
      value={form?.draft?.key ?? data.key}
    />
    <input
      type="hidden"
      name="expected_version"
      value={form?.draft?.expected_version ?? data.record.version}
    />
    <label
      >Role<input
        name="role_label"
        maxlength="80"
        value={form?.draft?.role_label ?? data.participant.role_label}
      /></label
    >
    <label
      >Primary designation<select
        name="primary"
        value={form?.draft?.primary ?? 'keep'}
        ><option value="keep">Keep current designation</option><option
          value="yes">Make primary</option
        ><option value="no">Make secondary</option></select
      ></label
    >
    <label
      >Association state<select
        name="state"
        value={form?.draft?.state ?? 'keep'}
        ><option value="keep">Keep current state</option><option value="end"
          >End association</option
        ><option value="restore">Restore association</option></select
      ></label
    >
    <p class="muted">
      Ending preserves history. Restoration requires active records; primary
      promotion requires workspace-wide reference visibility.
    </p>
    {#if form?.draft}<p>
        <a href="?">Discard draft and reload saved values</a>
      </p>{/if}
    <button>Save association</button>
  </form>
</section>
<section>
  <h2>Association history</h2>
  <ol>
    {#each data.history.items as event}<li>
        {event.role_label || 'Participant'} · {event.ended
          ? 'Ended'
          : 'Active'}{event.is_primary ? ' · Primary' : ''}
        <p class="muted">{event.occurred_at}</p>
      </li>{/each}
  </ol>
  {#if data.history.next_cursor}<a href={`?after=${data.history.next_cursor}`}
      >More history →</a
    >{/if}
</section>
