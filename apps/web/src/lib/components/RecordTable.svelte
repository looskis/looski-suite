<script lang="ts">
  import { page } from '$app/state';
  import DataTable from './table/DataTable.svelte';
  import type { TableColumn } from '$lib/table-layout';
  import RecordName from '$lib/components/ui/RecordName.svelte';
  import {
    type Field,
    type RecordRow,
    type ViewConfig,
    fieldValue,
    displayValue,
  } from '$lib/records';
  let {
    items,
    fields,
    config,
    root,
  }: { items: RecordRow[]; fields: Field[]; config: ViewConfig; root: string } =
    $props();
  const columns = $derived<TableColumn[]>([
    ...config.columns.map((key) => ({
      key,
      label: fields.find((f) => f.key === key)?.name ?? key,
      extraWidth: key === 'name' ? 42 : 0,
      minWidth: key === 'name' ? 160 : 88,
      sort:
        config.sort === key
          ? config.direction === 'asc'
            ? ('ascending' as const)
            : ('descending' as const)
          : undefined,
    })),
    ...(!config.columns.includes('name')
      ? [{ key: '__open', label: 'Record', minWidth: 88 }]
      : []),
  ]);
  const storageKey = $derived(
    `looski:columns:v1:${page.data.accountEmail ?? ''}:${root}:${page.url.searchParams.get('view') ?? 'all'}`,
  );
  function sortUrl(key: string) {
    const query = new URLSearchParams(page.url.searchParams);
    query.delete('after');
    query.set('sort', key);
    query.set(
      'direction',
      config.sort === key && config.direction === 'asc' ? 'desc' : 'asc',
    );
    return `?${query}`;
  }
</script>

<DataTable
  rows={items}
  {columns}
  {storageKey}
  label={root.endsWith('/contacts') ? 'Contacts' : 'Organizations'}
>
  {#snippet header(column)}{#if column.key === '__open'}Record{:else}<a
        class="sort-heading"
        href={sortUrl(column.key)}
        >{column.label}<span aria-hidden="true"
          >{config.sort === column.key
            ? config.direction === 'asc'
              ? '↑'
              : '↓'
            : '↕'}</span
        ></a
      >{/if}{/snippet}
  {#snippet cell(item, column)}{@const key = column.key}{@const field =
      fields.find((f) => f.key === key)}{@const value = fieldValue(item, key)}
    {#if key === '__open'}<a href={`${root}/${item.id}`}>Open ↗</a>
    {:else if key === 'name'}<RecordName
        name={String(value ?? 'Untitled')}
        href={`${root}/${item.id}`}
        kind={item.display_name !== undefined ? 'person' : 'organization'}
      />
    {:else if key === 'status' && value}<span
        class="status-pill"
        data-status={String(value)}>{displayValue(value, field)}</span
      >
    {:else}<span
        class="cell-value"
        class:numeric={field?.field_type === 'number'}
        title={displayValue(value, field)}>{displayValue(value, field)}</span
      >{/if}
  {/snippet}
  {#snippet empty()}<div class="empty-state">
      <strong>No records found</strong>
      <p>Adjust your filters or add a record.</p>
    </div>{/snippet}
</DataTable>

<style>
  .sort-heading {
    display: flex;
    gap: 12px;
    justify-content: space-between;
    color: inherit;
    text-decoration: none;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sort-heading span {
    opacity: 0.5;
    flex-shrink: 0;
  }
  .cell-value {
    display: block;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .numeric {
    font-variant-numeric: tabular-nums;
  }
  .status-pill {
    display: inline-flex;
    border: 1px solid var(--border-default);
    border-radius: 5px;
    padding: 3px 7px;
    font-size: 11px;
    text-transform: capitalize;
    background: var(--surface-hover);
  }
  .status-pill[data-status='customer'] {
    background: color-mix(in srgb, #16a34a 10%, var(--surface-panel));
    color: var(--text-primary);
  }
  .status-pill[data-status='lead'] {
    background: color-mix(in srgb, #3b82f6 10%, var(--surface-panel));
  }
</style>
