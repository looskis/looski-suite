<script lang="ts">
  import { shortDate } from '$lib/format';
  import RecordDrawer from '$lib/components/ui/RecordDrawer.svelte';
  import { projectStatusLabel, taskStatusLabel } from '$lib/project-labels';
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const root = $derived(
    `/workspaces/${data.workspace.id}/projects/${data.board.project.id}`,
  );
  const locked = $derived(data.board.project.archived);
  // Dragging only fills in the same form the buttons use; without scripting,
  // every move still works through those buttons.
  let dragForm: HTMLFormElement | undefined = $state();
  let drag = $state({ card: '', version: 0, column: '', before: '' });
  let dragging = $state('');
  function drop(column: string, before: string) {
    if (!dragging) return;
    const card = data.board.columns
      .flatMap((c) => c.cards)
      .find((c) => c.id === dragging);
    dragging = '';
    if (!card || card.id === before) return;
    drag = { card: card.id, version: card.version, column, before };
    queueMicrotask(() => dragForm?.requestSubmit());
  }
</script>

<svelte:head><title>{data.board.project.name} · Looski</title></svelte:head>
<h1>{data.board.project.name}</h1>
<p>
  {projectStatusLabel(data.board.project.status)}{#if data.board.project.due_on}
    · due {shortDate(
      data.board.project.due_on,
    )}{/if}{#if data.board.project.archived}
    · archived{/if}
</p>
{#if data.board.project.description}<p class="description">
    {data.board.project.description}
  </p>{/if}
{#if data.board.project.source_deal_id}<p>
    Created from <a
      href={`/workspaces/${data.workspace.id}/crm/deals/${data.board.project.source_deal_id}`}
      >a deal</a
    >.
  </p>{/if}
{#if form?.message}<p role="alert">{form.message}</p>{/if}
{#if !locked}
  <form bind:this={dragForm} method="POST" action="?/move" hidden>
    <input type="hidden" name="idempotency_key" value={`${data.key}:drag`} />
    <input type="hidden" name="card_id" value={drag.card} />
    <input type="hidden" name="expected_version" value={drag.version} />
    <input type="hidden" name="column_id" value={drag.column} />
    <input type="hidden" name="before_card_id" value={drag.before} />
  </form>
{/if}
<div class="board">
  {#each data.board.columns as column, c}
    <section
      class="column"
      aria-label={column.name}
      ondragover={(e) => {
        if (dragging) e.preventDefault();
      }}
      ondrop={(e) => {
        e.preventDefault();
        drop(column.id, '');
      }}
    >
      <h2>{column.name} <small>{column.cards.length}</small></h2>
      {#if column.task_status}<p class="hint">
          Moving here marks tasks {taskStatusLabel(
            column.task_status,
          ).toLowerCase()}.
        </p>{/if}
      {#each column.cards as card, i}
        <article
          class="card"
          draggable={!locked}
          ondragstart={() => (dragging = card.id)}
          ondragend={() => (dragging = '')}
          ondragover={(e) => {
            if (dragging) e.preventDefault();
          }}
          ondrop={(e) => {
            e.preventDefault();
            e.stopPropagation();
            drop(column.id, card.id);
          }}
        >
          <a class="title" href={`${root}/cards/${card.id}`}
            >{card.task.title}</a
          >
          <dl>
            <dt>Assignee</dt>
            <dd>
              {card.assignee_name ?? 'Outside this project'}
            </dd>
            <dt>Deadline</dt>
            <dd>
              {card.task.due_on ? shortDate(card.task.due_on) : 'No deadline'}
            </dd>
            <dt>Status</dt>
            <dd>{taskStatusLabel(card.task.status)}</dd>
            {#if card.comment_count}<dt>Comments</dt>
              <dd>{card.comment_count}</dd>{/if}
          </dl>
          {#if !locked}
            <div class="moves">
              {#if i > 0}
                <form method="POST" action="?/move">
                  <input
                    type="hidden"
                    name="idempotency_key"
                    value={`${data.key}:${card.id}:up`}
                  />
                  <input type="hidden" name="card_id" value={card.id} />
                  <input
                    type="hidden"
                    name="expected_version"
                    value={card.version}
                  />
                  <input type="hidden" name="column_id" value={column.id} />
                  <input
                    type="hidden"
                    name="before_card_id"
                    value={column.cards[i - 1].id}
                  />
                  <button
                    class="secondary"
                    aria-label={`Move ${card.task.title} up`}>↑</button
                  >
                </form>
              {/if}
              {#if i < column.cards.length - 1}
                <form method="POST" action="?/move">
                  <input
                    type="hidden"
                    name="idempotency_key"
                    value={`${data.key}:${card.id}:down`}
                  />
                  <input type="hidden" name="card_id" value={card.id} />
                  <input
                    type="hidden"
                    name="expected_version"
                    value={card.version}
                  />
                  <input type="hidden" name="column_id" value={column.id} />
                  <input
                    type="hidden"
                    name="before_card_id"
                    value={column.cards[i + 2]?.id ?? ''}
                  />
                  <button
                    class="secondary"
                    aria-label={`Move ${card.task.title} down`}>↓</button
                  >
                </form>
              {/if}
              <form method="POST" action="?/move" class="to-column">
                <input
                  type="hidden"
                  name="idempotency_key"
                  value={`${data.key}:${card.id}:column`}
                />
                <input type="hidden" name="card_id" value={card.id} />
                <input
                  type="hidden"
                  name="expected_version"
                  value={card.version}
                />
                <select
                  name="column_id"
                  aria-label={`Column for ${card.task.title}`}
                >
                  {#each data.board.columns as target, t}
                    <option value={target.id} selected={t === c}
                      >{target.name}</option
                    >
                  {/each}
                </select>
                <button class="secondary">Move</button>
              </form>
            </div>
          {/if}
        </article>
      {:else}
        <p class="hint">No tasks you can see.</p>
      {/each}
    </section>
  {/each}
</div>
{#if !locked}
  <RecordDrawer title="Add a task" error={Boolean(form?.input)}>
    <form method="POST" action="?/add">
      <input
        type="hidden"
        name="idempotency_key"
        value={form?.key ?? `${data.key}:add`}
      />
      <label
        >Title<input
          name="title"
          required
          maxlength="200"
          value={form?.input?.title ?? ''}
        /></label
      >
      <label
        >Column<select name="column_id">
          {#each data.board.columns as column}<option value={column.id}
              >{column.name}</option
            >{/each}
        </select></label
      >
      <label
        >Assignee<select name="assigned_membership_id">
          <option value="">Me</option>
          {#each data.members as member}<option
              value={member.id}
              selected={form?.input?.assigned_membership_id === member.id}
              >{member.display_name}</option
            >{/each}
        </select></label
      >
      <label
        >Deadline<input
          name="due_on"
          type="date"
          value={form?.input?.due_on ?? ''}
        /></label
      >
      <button>Add task</button>
    </form>
    {#if data.tasks.length}
      <h3>Add an existing task</h3>
      <form method="POST" action="?/link" class="inline">
        <input
          type="hidden"
          name="idempotency_key"
          value={`${data.key}:link`}
        />
        <label
          >Task<select name="task_id">
            {#each data.tasks as task}<option value={task.id}
                >{task.title}</option
              >{/each}
          </select></label
        >
        <button class="secondary">Add to board</button>
      </form>
    {/if}
  </RecordDrawer>
{/if}

<style>
  .board {
    display: flex;
    gap: var(--space-3-5);
    align-items: flex-start;
    overflow-x: auto;
    padding-bottom: var(--space-3);
  }
  .column {
    flex: 0 0 300px;
    border: 0;
    min-height: 360px;
    margin: 0;
    padding: var(--space-3);
    background: var(--surface-board);
    border-radius: var(--radius-md);
  }
  .column h2 {
    font-size: 12px;
    padding: 4px 0 12px;
    margin: 0 0 var(--space-1-5);
  }
  .column h2 small {
    color: var(--text-secondary);
    font-weight: var(--font-weight-regular);
  }
  .hint {
    font-size: 11px;
    color: var(--text-secondary);
    margin: 0 0 var(--space-2);
  }
  .card {
    background: var(--surface-panel);
    border: 1px solid var(--border-default);
    border-radius: 9px;
    box-shadow: 0 1px 3px #00000005;
    padding: 16px;
    margin-bottom: var(--space-2-5);
  }
  .card[draggable='true'] {
    cursor: grab;
  }
  .title {
    font-weight: var(--font-weight-semibold);
    overflow-wrap: anywhere;
  }
  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: var(--space-0-5) var(--space-2);
    margin: var(--space-2) 0;
    font-size: 11px;
  }
  dt {
    color: var(--text-secondary);
  }
  dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
  .moves {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
    align-items: center;
  }
  .moves form {
    display: flex;
    gap: var(--space-1);
    margin: 0;
  }
  .moves button,
  .moves select {
    padding: var(--space-0-5) var(--space-2);
    font-size: 11px;
    width: auto;
    min-height: 0;
  }
  .description {
    white-space: pre-wrap;
  }
</style>
