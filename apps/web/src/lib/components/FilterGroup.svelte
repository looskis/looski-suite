<script lang="ts">
  import FilterGroup from './FilterGroup.svelte';
  import { type Field, type Filter, operators } from '$lib/records';
  let {
    node,
    fields,
    onchange,
    depth = 0,
  }: {
    node: Filter;
    fields: Field[];
    onchange: (node: Filter) => void;
    depth?: number;
  } = $props();
  const condition = (): Filter => ({
    type: 'condition',
    field: fields.find((f) => f.key === 'name')?.key ?? fields[0].key,
    operator: 'contains',
    value: '',
  });
  function setValue(raw: string, field?: Field, multiple = false) {
    const convert = (s: string) =>
      field?.field_type === 'number'
        ? Number(s)
        : field?.field_type === 'boolean'
          ? s === 'true'
          : s;
    return multiple
      ? raw.split(',').map((s) => convert(s.trim()))
      : convert(raw);
  }
</script>

{#if node.type === 'group'}
  <div class="filter-group">
    <div class="group-heading">
      <span>Match</span><select
        aria-label="Group match"
        value={node.operator}
        onchange={(e) => onchange({ ...node, operator: e.currentTarget.value })}
        ><option value="and">All conditions (AND)</option><option value="or"
          >Any condition (OR)</option
        ></select
      >
    </div>
    {#each node.rules as rule, i}<div class="filter-rule">
        <FilterGroup
          node={rule}
          {fields}
          depth={depth + 1}
          onchange={(next) => {
            if (node.type === 'group')
              onchange({
                ...node,
                rules: node.rules.map((r, j) => (j === i ? next : r)),
              });
          }}
        /><button
          type="button"
          class="remove-rule"
          aria-label="Remove condition or group"
          onclick={() => {
            if (node.type === 'group')
              onchange({
                ...node,
                rules: node.rules.filter((_, j) => j !== i),
              });
          }}>×</button
        >
      </div>{/each}
    <div class="group-actions">
      <button
        type="button"
        class="secondary"
        disabled={node.rules.length >= 20}
        onclick={() => {
          if (node.type === 'group')
            onchange({ ...node, rules: [...node.rules, condition()] });
        }}>+ Condition</button
      >{#if depth < 4}<button
          type="button"
          class="secondary"
          disabled={node.rules.length >= 20}
          onclick={() => {
            if (node.type === 'group')
              onchange({
                ...node,
                rules: [
                  ...node.rules,
                  { type: 'group', operator: 'or', rules: [condition()] },
                ],
              });
          }}>+ Group</button
        >{/if}
    </div>
  </div>
{:else}
  {@const field = fields.find((f) => f.key === node.field)}
  <div class="condition">
    <select
      aria-label="Filter field"
      value={node.field}
      onchange={(e) =>
        onchange({
          type: 'condition',
          field: e.currentTarget.value,
          operator: 'eq',
          value: '',
        })}
      >{#each fields as f}<option value={f.key}
          >{f.name}{f.archived ? ' (archived)' : ''}</option
        >{/each}</select
    >
    <select
      aria-label="Filter operator"
      value={node.operator}
      onchange={(e) =>
        onchange({
          ...node,
          operator: e.currentTarget.value,
          value: ['in', 'not_in'].includes(e.currentTarget.value) ? [] : '',
        })}
      >{#each operators(field) as [value, label]}<option {value}>{label}</option
        >{/each}</select
    >
    {#if !['is_empty', 'is_not_empty'].includes(node.operator)}
      {#if ['in', 'not_in'].includes(node.operator)}<input
          aria-label="Filter values"
          placeholder="Values, separated by commas"
          value={Array.isArray(node.value) ? node.value.join(', ') : ''}
          onchange={(e) =>
            onchange({
              ...node,
              value: setValue(e.currentTarget.value, field, true),
            })}
        />
      {:else if field?.field_type === 'select' || field?.field_type === 'boolean'}<select
          aria-label="Filter value"
          value={String(node.value ?? '')}
          onchange={(e) =>
            onchange({
              ...node,
              value: setValue(e.currentTarget.value, field),
            })}
          ><option value="" disabled>Select value</option
          >{#each field.field_type === 'boolean' ? ['true', 'false'] : field.options as value}<option
              {value}
              >{value === 'true'
                ? 'Yes'
                : value === 'false'
                  ? 'No'
                  : value}</option
            >{/each}</select
        >
      {:else}<input
          aria-label="Filter value"
          type={field?.field_type === 'number'
            ? 'number'
            : field?.field_type === 'date'
              ? 'date'
              : 'text'}
          step="any"
          value={String(node.value ?? '')}
          onchange={(e) =>
            onchange({
              ...node,
              value: setValue(e.currentTarget.value, field),
            })}
        />{/if}{/if}
  </div>
{/if}

<style>
  .filter-group {
    padding: 12px;
    border: 1px solid var(--border-default);
    border-radius: 8px;
    background: var(--surface-hover);
  }
  .group-heading,
  .group-actions {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .group-heading {
    font-size: 12px;
    margin-bottom: 12px;
  }
  .group-heading select {
    width: auto;
  }
  .filter-rule {
    display: flex;
    gap: 6px;
    margin: 8px 0;
    align-items: start;
  }
  .filter-rule > :global(:first-child) {
    flex: 1;
    min-width: 0;
  }
  .condition {
    display: grid;
    grid-template-columns: 1fr 1fr 1.2fr;
    gap: 6px;
  }
  .condition select,
  .condition input {
    margin: 0;
    min-width: 0;
    font-size: 12px;
  }
  .remove-rule {
    background: transparent;
    color: var(--text-secondary);
    padding: 6px 9px;
  }
  .group-actions {
    margin-top: 12px;
  }
  .group-actions button {
    font-size: 11px;
  }
  @media (max-width: 700px) {
    .condition {
      grid-template-columns: 1fr;
    }
    .filter-group {
      padding: 8px;
    }
  }
</style>
