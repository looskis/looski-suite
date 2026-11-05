<script lang="ts">
  import { type Field, fieldValue, displayValue } from '$lib/records';
  let {
    fields,
    properties = {},
  }: { fields: Field[]; properties?: { [key: string]: unknown } } = $props();
</script>

<dl class="property-details">
  {#each fields.filter((f) => f.editable && !f.archived) as field}{@const value =
      fieldValue(
        { id: '', properties },
        field.key,
      )}{#if value !== undefined && value !== null && value !== ''}<div>
        <dt>{field.name}</dt>
        <dd>{displayValue(value, field)}</dd>
      </div>{/if}{/each}
</dl>

<style>
  .property-details {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 16px;
    margin: 20px 0;
  }
  .property-details dt {
    font-size: 11px;
    color: var(--text-secondary);
    margin-bottom: 5px;
  }
  .property-details dd {
    margin: 0;
    font-size: 13px;
    overflow-wrap: anywhere;
  }
</style>
