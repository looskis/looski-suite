<script lang="ts">
  import { projectStatuses, taskStatuses } from '$lib/project-labels';
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const draft = $derived(form?.input ?? data.project);
</script>

<svelte:head><title>Settings · {data.project.name} · Looski</title></svelte:head
>
<h1>Settings</h1>
{#if form?.message}<p role="alert">{form.message}</p>{/if}
<section>
  <h2>Project</h2>
  <form method="POST" action="?/save">
    <input type="hidden" name="idempotency_key" value={`${data.key}:save`} />
    <input type="hidden" name="expected_version" value={data.project.version} />
    <label
      >Name<input
        name="name"
        required
        maxlength="120"
        value={draft.name}
      /></label
    >
    <label
      >Description<textarea
        name="description"
        maxlength="4000"
        rows="3"
        value={draft.description}></textarea></label
    >
    <label
      >Status<select name="status">
        {#each projectStatuses as status}<option
            value={status.value}
            selected={draft.status === status.value}>{status.label}</option
          >{/each}
      </select></label
    >
    <label
      >Deadline<input
        name="due_on"
        type="date"
        value={draft.due_on ?? ''}
      /></label
    >
    <button disabled={data.project.archived}>Save project</button>
  </form>
</section>
<section>
  <h2>Board columns</h2>
  <p>
    A column with a task status asks Tasks to set that status when a card moves
    into it. Changing a task elsewhere does not move its card.
  </p>
  {#each data.columns as column, i}
    <form method="POST" action="?/saveColumn" class="inline">
      <input
        type="hidden"
        name="idempotency_key"
        value={`${data.key}:column:${column.id}`}
      />
      <input type="hidden" name="column_id" value={column.id} />
      <input type="hidden" name="expected_version" value={column.version} />
      <label
        >Name<input
          name="name"
          required
          maxlength="60"
          value={column.name}
        /></label
      >
      <label
        >Sets task status<select name="task_status">
          <option value="">Nothing</option>
          {#each taskStatuses as status}<option
              value={status.value}
              selected={column.task_status === status.value}
              >{status.label}</option
            >{/each}
        </select></label
      >
      <label
        >Place<input
          name="position"
          type="number"
          min="1"
          max={data.columns.length}
          value={i + 1}
        /></label
      >
      <button class="secondary">Save column</button>
    </form>
    {#if data.columns.length > 1}
      <form method="POST" action="?/removeColumn" class="inline">
        <input
          type="hidden"
          name="idempotency_key"
          value={`${data.key}:remove:${column.id}`}
        />
        <input type="hidden" name="column_id" value={column.id} />
        <input type="hidden" name="expected_version" value={column.version} />
        <label
          >Move its cards to<select name="move_cards_to">
            {#each data.columns.filter((c) => c.id !== column.id) as target}<option
                value={target.id}>{target.name}</option
              >{/each}
          </select></label
        >
        <button class="secondary">Remove {column.name}</button>
      </form>
    {/if}
  {/each}
  <h3>Add a column</h3>
  <form method="POST" action="?/addColumn" class="inline">
    <input type="hidden" name="idempotency_key" value={`${data.key}:add`} />
    <label>Name<input name="name" required maxlength="60" /></label>
    <label
      >Sets task status<select name="task_status">
        <option value="">Nothing</option>
        {#each taskStatuses as status}<option value={status.value}
            >{status.label}</option
          >{/each}
      </select></label
    >
    <button>Add column</button>
  </form>
</section>
<section>
  <h2>{data.project.archived ? 'Restore' : 'Archive'}</h2>
  <p>
    {data.project.archived
      ? 'Restoring lets the board change again.'
      : 'Archiving freezes the board and hides the project from lists. Its tasks stay in Tasks.'}
  </p>
  <form method="POST" action="?/archive">
    <input type="hidden" name="idempotency_key" value={`${data.key}:archive`} />
    <input type="hidden" name="expected_version" value={data.project.version} />
    <input
      type="hidden"
      name="archived"
      value={String(!data.project.archived)}
    />
    <button class="secondary"
      >{data.project.archived ? 'Restore project' : 'Archive project'}</button
    >
  </form>
</section>
