<script lang="ts">
  import TaskFields from './TaskFields.svelte';
  import type { TaskLinksData, TaskLinkFormResult } from '$lib/crm-task-links';
  let {
    data,
    form,
  }: {
    data: TaskLinksData;
    form?: TaskLinkFormResult | null;
  } = $props();
  const root = $derived(
    `/workspaces/${data.workspace.id}/crm/${data.sales}/${data.record.id}`,
  );
  const creating = $derived(
    form?.draft?.mode === 'create' ? form.draft : undefined,
  );
  const linking = $derived(
    form?.draft?.mode === 'link' ? form.draft : undefined,
  );
  const missingEndLink = $derived(
    !data.links.items.some((link) => link.id === form?.endDraft?.link_id),
  );
  const missingSelection = $derived(
    !data.choices.some((task) => task.id === linking?.task_id),
  );
</script>

<header>
  <a href={root}>← {data.record.title}</a><a
    href={`/workspaces/${data.workspace.id}/tasks`}>All tasks</a
  >
</header>
<h1>Follow-up tasks</h1>
<p>
  Task assignment and status remain independent of this {data.sales === 'deals'
    ? 'deal'
    : 'opportunity'}.
</p>
{#if form?.message}<p class="error" role="alert">{form.message}</p>{/if}
{#if form?.draft || form?.endDraft}<p>
    <a href={root + '/tasks'}>Discard draft and reload saved values</a>
  </p>{/if}
<p>
  <a href={`?ended=${!data.ended}`}
    >{data.ended ? 'Show active links' : 'Show ended links'}</a
  >
</p>
<section>
  <h2>{data.ended ? 'Ended links' : 'Linked tasks'}</h2>
  {#each data.links.items as link}<article>
      <h3>
        <a href={`/workspaces/${data.workspace.id}/tasks/${link.task.id}`}
          >{link.task.title}</a
        >
      </h3>
      <p>
        {link.task.status.replaceAll('_', ' ')} · Due {link.task.due_on ??
          'unscheduled'}{link.task.archived ? ' · archived task' : ''}
      </p>
      <a href={root + '/tasks/' + link.id}>Link history</a>
      <form method="POST" action="?/end">
        <input type="hidden" name="link_id" value={link.id} /><input
          type="hidden"
          name="idempotency_key"
          value={form?.endDraft?.link_id === link.id
            ? form.endDraft.key
            : data.endKeys[link.id]}
        /><input
          type="hidden"
          name="expected_version"
          value={form?.endDraft?.link_id === link.id
            ? form.endDraft.expected_version
            : data.record.version}
        /><input
          type="hidden"
          name="ended"
          value={form?.endDraft?.link_id === link.id
            ? form.endDraft.ended
            : String(!link.ended)}
        /><button class="secondary"
          >{link.ended ? 'Restore link' : 'End link'}</button
        >
      </form>
    </article>{/each}
  {#if !data.links.items.length}<p>No visible task links.</p>{/if}
  {#if form?.endDraft && missingEndLink}<p>
      The selected link is no longer on this page. Reload its saved state before
      retrying.
    </p>
    <form method="POST" action="?/end">
      <input type="hidden" name="link_id" value={form.endDraft.link_id} /><input
        type="hidden"
        name="idempotency_key"
        value={form.endDraft.key}
      /><input
        type="hidden"
        name="expected_version"
        value={form.endDraft.expected_version}
      /><input type="hidden" name="ended" value={form.endDraft.ended} /><button
        class="secondary">Retry saved link change</button
      >
    </form>{/if}
  {#if data.links.next_cursor}<a
      href={`?after=${data.links.next_cursor}&ended=${data.ended}&task_search=${encodeURIComponent(data.search)}`}
      >More task links →</a
    >{/if}
</section>
<section>
  <h2>Create follow-up</h2>
  <form method="POST" action="?/create">
    <input
      type="hidden"
      name="idempotency_key"
      value={creating?.key ?? data.createKey}
    /><input
      type="hidden"
      name="expected_version"
      value={creating?.expected_version ?? data.record.version}
    /><TaskFields fields={creating ?? data.fields} creating /><button
      disabled={data.record.archived}>Create task and link</button
    >
  </form>
</section>
<section>
  <h2>Link an existing task</h2>
  <form method="GET">
    <input type="hidden" name="ended" value={String(data.ended)} /><label
      >Find task<input name="task_search" value={data.search} /></label
    ><button class="secondary">Search tasks</button>
  </form>
  <p>
    Showing up to 100 accessible active tasks. Refine the search to find another
    task.
  </p>
  <form method="POST" action="?/link">
    <input
      type="hidden"
      name="idempotency_key"
      value={linking?.key ?? data.linkKey}
    /><input
      type="hidden"
      name="expected_version"
      value={linking?.expected_version ?? data.record.version}
    /><label
      >Task<select name="task_id" value={linking?.task_id ?? ''} required
        ><option value="">Choose task</option
        >{#if linking?.task_id && missingSelection}<option
            value={linking.task_id}>Previously selected task</option
          >{/if}{#each data.choices as task}<option value={task.id}
            >{task.title}</option
          >{/each}</select
      ></label
    ><button disabled={data.record.archived}>Link task</button>
  </form>
  {#if data.record.archived}<p>
      Restore this record before adding tasks or links.
    </p>{/if}
</section>
