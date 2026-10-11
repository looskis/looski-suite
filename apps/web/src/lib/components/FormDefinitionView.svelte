<script lang="ts">
  import { kindLabel, type Definition } from '$lib/form-definition';
  let { definition }: { definition: Definition } = $props();
  const conditionText = (field: Definition['fields'][number]) => {
    const source = definition.fields.find(
      (f) => f.key === field.show_if?.field,
    );
    const values = field.show_if?.equals.map(
      (v) => source?.options?.find((o) => o.key === v)?.label ?? v,
    );
    return `Shown when “${source?.label ?? field.show_if?.field}” is ${values?.join(' or ')}`;
  };
</script>

<ol>
  {#each definition.fields as field}
    <li>
      <strong>{field.label}</strong>
      <small
        >{kindLabel(field.kind)}{field.required ? ' · required' : ''} · key
        <code>{field.key}</code></small
      >
      {#if field.help}<br /><small>{field.help}</small>{/if}
      {#if field.options?.length}<br /><small
          >Options: {field.options.map((o) => o.label).join(', ')}</small
        >{/if}
      {#if field.show_if}<br /><small>{conditionText(field)}</small>{/if}
    </li>
  {:else}<li>No questions.</li>{/each}
</ol>
