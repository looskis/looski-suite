<script lang="ts">
  let { data, form } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}`);
  function link(kind: string, id: string) {
    if (kind === 'task-types') return `${root}/tasks/types/${id}`;
    if (kind === 'task-templates') return `${root}/tasks/templates/${id}`;
    if (['pipelines', 'stages', 'categories', 'statuses'].includes(kind))
      return `${root}/crm/configuration/${kind}/${id}`;
    return `${root}/crm/${kind}/${id}`;
  }
</script>

<header><a href={`${root}/crm/starter-packages`}>← Starter package</a></header>
<h1>General CRM · version {data.installation.package_version}</h1>
<p>
  {data.installation.removed ? 'Removed' : 'Active'} · installed {data
    .installation.installed_at}
</p>
{#if form?.message}<p role="alert" class="error">{form.message}</p>
  <a href={`?reload=${Date.now()}`}>Discard draft and reload saved values</a
  >{/if}
<section>
  <h2>Copied definitions</h2>
  <ul>
    {#each data.installation.records as record}<li>
        <a href={link(record.kind, record.id)}>{record.name}</a> · {record.kind} ·
        {record.archived ? 'Archived' : 'Active'}
      </li>{/each}
  </ul>
</section>
{#if data.installation.removal}<section>
    <h2>Removal results</h2>
    <ul>
      {#each data.installation.removal as decision}<li>
          {data.installation.records.find((r) => r.id === decision.id)?.name ??
            decision.kind}: {decision.outcome.replaceAll(
            '_',
            ' ',
          )}{decision.reason
            ? ` (${decision.reason.replaceAll('_', ' ')})`
            : ''}
        </li>{/each}
    </ul>
  </section>{/if}
{#if !data.installation.removed || form?.draft}<section>
    <h2>Remove package</h2>
    <p>
      Archive unchanged defaults. Customized definitions and definitions
      required by active configuration are retained. Business records, tasks and
      history remain available.
    </p>
    <form method="POST">
      <input
        type="hidden"
        name="idempotency_key"
        value={form?.draft?.key ?? data.key}
      /><input
        type="hidden"
        name="expected_version"
        value={form?.draft?.expected_version ?? data.installation.version}
      /><button class="secondary">Remove starter package</button>
    </form>
  </section>{/if}
