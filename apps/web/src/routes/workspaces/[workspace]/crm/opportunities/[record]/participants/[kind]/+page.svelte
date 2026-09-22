<script lang="ts">
  let { data, form } = $props();
</script>

<header>
  <a
    href={`/workspaces/${data.workspace.id}/crm/${data.sales}/${data.record.id}`}
    >← {data.record.title}</a
  ><span>Participants</span>
</header>
<h1>{data.kind === 'contacts' ? 'Contact' : 'Organization'} participants</h1>
<p>
  Associations do not grant access to contact records. Historical references
  survive merges.
</p>
{#if form?.message}<p class="error" role="alert">{form.message}</p>{/if}
<section>
  <a href={`?ended=${!data.ended}`}
    >{data.ended ? 'View active' : 'View ended'} associations</a
  >
  <ul>
    {#each data.page.items as participant}<li>
        <a
          href={`/workspaces/${data.workspace.id}/crm/${data.sales}/${data.record.id}/participants/${data.kind}/${participant.id}`}
          >{participant.name}</a
        >
        · {participant.role_label || 'Participant'}{participant.is_primary
          ? ' · Primary'
          : ''}{participant.reference_archived ? ' · Archived reference' : ''}
      </li>{:else}<li>No visible associations.</li>{/each}
  </ul>
  {#if data.page.next_cursor}<a
      href={`?ended=${data.ended}&after=${data.page.next_cursor}`}
      >More participants →</a
    >{/if}
</section>
<section>
  <h2>Add a participant</h2>
  <form method="GET" class="inline">
    <label>Find {data.kind}<input name="search" value={data.search} /></label
    ><button class="secondary">Search</button>
  </form>
  {#if data.moreChoices}<p>
      Showing 100 matches. Refine the search to find another record.
    </p>{/if}
  {#if !data.record.archived}<form method="POST">
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
        >Record<select
          name="reference_id"
          value={form?.draft?.reference_id ?? data.choices[0]?.id ?? ''}
          required
        >
          {#if form?.draft?.reference_id && !data.choices.some((c) => c.id === form.draft.reference_id)}<option
              value={form.draft.reference_id}
              >Submitted selection — review before saving</option
            >{/if}
          {#each data.choices as choice}<option value={choice.id}
              >{choice.name}</option
            >{/each}</select
        ></label
      >
      <label
        >Role (optional)<input
          name="role_label"
          maxlength="80"
          value={form?.draft?.role_label ?? ''}
          placeholder="Buyer, seller, adviser…"
        /></label
      >
      <label
        >Primary<select name="primary" value={form?.draft?.primary ?? 'no'}
          ><option value="no">Secondary participant</option><option value="yes"
            >Make primary</option
          ></select
        ></label
      >
      <p class="muted">
        Primary promotion requires workspace-wide visibility of this participant
        type. It replaces the previous primary and retains its history.
      </p>
      {#if form?.draft}<p><a href="?">Discard draft and reload</a></p>{/if}
      <button>Add participant</button>
    </form>{:else}<p>
      Restore this sales record before adding participants.
    </p>{/if}
</section>
