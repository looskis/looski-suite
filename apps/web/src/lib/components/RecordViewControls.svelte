<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import FilterGroup from './FilterGroup.svelte';
  import { filterProblem } from '$lib/records';
  import type { Field, Filter, ViewConfig, SavedView } from '$lib/records';
  let {
    fields,
    views,
    config,
    viewId,
    viewKey,
    kind,
    workspace,
    search,
    tagId,
  }: {
    fields: Field[];
    views: SavedView[];
    config: ViewConfig;
    viewId: string;
    viewKey: string;
    kind: string;
    workspace: string;
    search: string;
    tagId?: string;
  } = $props();
  let filterDialog: HTMLDialogElement;
  let columnsDialog: HTMLDialogElement;
  let saveDialog: HTMLDialogElement;
  let draft = $state<Filter>({ type: 'group', operator: 'and', rules: [] });
  let columns = $state<string[]>([]);
  const problem = $derived(filterProblem(draft, fields));
  const selected = $derived(views.find((v) => v.id === viewId));
  const preserved = $derived(
    [...page.url.searchParams.entries()].filter(
      ([key]) => !['search', 'after'].includes(key),
    ),
  );
  function update(changes: { [key: string]: string | null }) {
    const query = new URLSearchParams(page.url.searchParams);
    query.delete('after');
    for (const [key, value] of Object.entries(changes))
      value === null ? query.delete(key) : query.set(key, value);
    goto(`?${query}`, { keepFocus: true, noScroll: true });
  }
  function reorder(i: number, offset: number) {
    const next = [...columns];
    [next[i], next[i + offset]] = [next[i + offset], next[i]];
    columns = next;
  }
</script>

<nav class="view-tabs" aria-label="Saved views">
  <a href="?" aria-current={!viewId && !config.archived ? 'page' : undefined}
    >All {kind}</a
  ><a
    href="?archived=true"
    aria-current={!viewId && config.archived ? 'page' : undefined}>Archived</a
  >{#each views as view}<a
      href={`?view=${view.id}`}
      aria-current={viewId === view.id ? 'page' : undefined}
      >{view.name}{view.shared ? ' · Team' : ''}</a
    >{/each}
</nav>
<div class="record-toolbar">
  <form method="GET" role="search">
    <label class="sr-only" for="record-search">Search {kind}</label><input
      id="record-search"
      name="search"
      type="search"
      value={search}
      placeholder={`Search ${kind}…`}
    />{#each preserved as [key, value]}<input
        type="hidden"
        name={key}
        {value}
      />{/each}<button class="secondary">Search</button>
  </form>
  <div class="record-tools">
    <button
      type="button"
      class="secondary"
      class:active={Boolean(config.filter)}
      onclick={() => {
        draft = structuredClone(
          config.filter ?? { type: 'group', operator: 'and', rules: [] },
        );
        filterDialog.showModal();
      }}>Filters{config.filter ? ' •' : ''}</button
    ><button
      type="button"
      class="secondary"
      onclick={() => {
        columns = [...config.columns];
        columnsDialog.showModal();
      }}>Columns <span class="muted">{config.columns.length}</span></button
    ><button
      type="button"
      class="secondary"
      onclick={() => saveDialog.showModal()}>Save view</button
    ><a
      class="field-settings"
      href={`/workspaces/${workspace}/settings/fields?kind=${kind}`}
      aria-label="Manage record fields">Manage fields ↗</a
    >
  </div>
</div>
{#if config.filter || search || tagId}<div class="active-query">
    {#if config.filter}<span>Advanced filters applied</span><button
        type="button"
        onclick={() => update({ filter: 'null' })}>Clear filters</button
      >{/if}{#if search}<button
        type="button"
        onclick={() => update({ search: null })}>Clear search “{search}”</button
      >{/if}{#if tagId}<button
        type="button"
        onclick={() => update({ tag_id: null })}>Clear tag</button
      >{/if}
  </div>{/if}
<dialog
  aria-label="Advanced filters"
  bind:this={filterDialog}
  class="view-dialog filters"
>
  <div class="dialog-heading">
    <h2>Filter {kind}</h2>
    <button
      type="button"
      class="secondary"
      onclick={() => filterDialog.close()}
      aria-label="Close filters">×</button
    >
  </div>
  {#if problem}<p role="alert" class="error">{problem}</p>{/if}
  <FilterGroup node={draft} {fields} onchange={(value) => (draft = value)} />
  <div class="dialog-footer">
    <button
      class="secondary"
      type="button"
      onclick={() => {
        update({ filter: 'null' });
        filterDialog.close();
      }}>Clear filters</button
    ><button
      disabled={Boolean(problem)}
      type="button"
      onclick={() => {
        update({
          filter: JSON.stringify(
            draft.type === 'group' && !draft.rules.length ? null : draft,
          ),
        });
        filterDialog.close();
      }}>Apply filters</button
    >
  </div>
</dialog>
<dialog
  aria-label="Configure columns"
  bind:this={columnsDialog}
  class="view-dialog"
>
  <div class="dialog-heading">
    <h2>Columns</h2>
    <button
      type="button"
      class="secondary"
      onclick={() => columnsDialog.close()}
      aria-label="Close columns">×</button
    >
  </div>
  <p class="muted">
    Choose up to 40 fields. Use the arrows to change their order.
  </p>
  <ol class="chosen-columns">
    {#each columns as key, i}<li>
        <span>{fields.find((f) => f.key === key)?.name}</span><button
          type="button"
          class="secondary"
          aria-label={`Move ${key} left`}
          disabled={i === 0}
          onclick={() => reorder(i, -1)}>↑</button
        ><button
          type="button"
          class="secondary"
          aria-label={`Move ${key} right`}
          disabled={i === columns.length - 1}
          onclick={() => reorder(i, 1)}>↓</button
        ><button
          type="button"
          class="secondary"
          aria-label={`Remove ${key} column`}
          disabled={columns.length === 1}
          onclick={() => (columns = columns.filter((c) => c !== key))}>×</button
        >
      </li>{/each}
  </ol>
  <label
    >Add column<select
      aria-label="Add column"
      value=""
      onchange={(e) => {
        if (e.currentTarget.value && columns.length < 40)
          columns = [...columns, e.currentTarget.value];
        e.currentTarget.value = '';
      }}
      ><option value="">Choose a field…</option
      >{#each fields.filter((f) => !columns.includes(f.key) && !f.archived) as field}<option
          value={field.key}>{field.name}</option
        >{/each}</select
    ></label
  >
  <div class="dialog-footer">
    <button
      type="button"
      onclick={() => {
        update({ columns: columns.join(',') });
        columnsDialog.close();
      }}>Apply columns</button
    >
  </div>
</dialog>
<dialog aria-label="Save view" bind:this={saveDialog} class="view-dialog">
  <div class="dialog-heading">
    <h2>Save view</h2>
    <button
      type="button"
      class="secondary"
      onclick={() => saveDialog.close()}
      aria-label="Close save view">×</button
    >
  </div>
  <form method="POST" action="?/saveView">
    <input type="hidden" name="view_key" value={viewKey} /><input
      type="hidden"
      name="config"
      value={JSON.stringify(config)}
    /><label
      >View name<input
        name="name"
        maxlength="80"
        required
        value={selected?.name ?? ''}
      /></label
    ><label class="check"
      ><input
        type="checkbox"
        name="shared"
        checked={selected?.shared ?? false}
      />Share with workspace</label
    >{#if selected?.mine}<label
        >Save as<select name="view_id"
          ><option value="">New view</option><option value={selected.id}
            >Update “{selected.name}”</option
          ></select
        ></label
      ><input type="hidden" name="version" value={selected.version} />{/if}
    <p class="muted">
      Saves columns, filters, archive state and sorting. Search text stays
      temporary.
    </p>
    <div class="dialog-footer"><button>Save view</button></div>
  </form>
  {#if selected?.mine}<form
      method="POST"
      action="?/deleteView"
      class="delete-view"
    >
      <input type="hidden" name="view_id" value={selected.id} /><input
        type="hidden"
        name="version"
        value={selected.version}
      /><input
        type="hidden"
        name="view_key"
        value={`${viewKey}:delete`}
      /><button class="secondary">Delete this saved view</button>
    </form>{/if}
</dialog>

<style>
  .delete-view {
    margin-top: 24px;
    padding-top: 16px;
    border-top: 1px solid var(--border-default);
  }
  .record-toolbar,
  .record-tools,
  .record-toolbar form {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .record-toolbar {
    justify-content: space-between;
    flex-wrap: wrap;
    margin: 16px 0;
  }
  .record-toolbar form {
    flex: 1;
    max-width: 420px;
  }
  .record-toolbar button {
    flex-shrink: 0;
    white-space: nowrap;
  }
  .record-toolbar input {
    flex: 1;
    min-width: 0;
    width: 0;
    margin: 0;
  }
  .record-tools {
    flex-wrap: wrap;
  }
  .field-settings {
    font-size: 11px;
    color: var(--text-secondary);
    margin-left: 4px;
  }
  .active-query {
    display: flex;
    gap: 12px;
    align-items: center;
    font-size: 11px;
    margin: 10px 0;
  }
  .active-query button {
    padding: 0;
    background: transparent;
    color: var(--text-secondary);
    text-decoration: underline;
    font-size: 11px;
  }
  .view-dialog {
    width: min(560px, calc(100vw - 32px));
    max-height: 85vh;
    border: 1px solid var(--border-default);
    border-radius: 12px;
    padding: 24px;
    background: var(--surface-panel);
    color: var(--text-primary);
    box-shadow: 0 24px 80px #0003;
  }
  .view-dialog.filters {
    width: min(900px, calc(100vw - 32px));
  }
  .view-dialog::backdrop {
    background: #10182866;
  }
  .dialog-heading,
  .dialog-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .dialog-heading {
    margin-bottom: 20px;
  }
  .dialog-heading h2 {
    margin: 0;
  }
  .dialog-footer {
    justify-content: flex-end;
    margin-top: 20px;
  }
  .chosen-columns {
    list-style: none;
    padding: 0;
  }
  .chosen-columns li {
    display: flex;
    gap: 5px;
    align-items: center;
    padding: 7px 0;
    border-bottom: 1px solid var(--border-default);
  }
  .chosen-columns span {
    flex: 1;
    font-size: 13px;
  }
  .chosen-columns button {
    padding: 3px 8px;
  }
  .active {
    border-color: var(--text-primary) !important;
  }
</style>
