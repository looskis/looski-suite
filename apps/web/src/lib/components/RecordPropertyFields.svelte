<script lang="ts">
  import { countries, type Field, fieldValue } from '$lib/records';
  let {
    fields,
    properties = {},
  }: { fields: Field[]; properties?: { [key: string]: unknown } | null } =
    $props();
  const value = (key: string) =>
    fieldValue({ id: '', properties: properties ?? {} }, key);
</script>

<input type="hidden" name="properties_present" value="true" />
<input
  type="hidden"
  name="properties_original"
  value={JSON.stringify(properties ?? {})}
/>
<div class="property-grid">
  {#each fields.filter((f) => f.editable && !f.archived && !f.key.startsWith('location.')) as field}
    <label
      >{field.name}
      {#if field.field_type === 'select' || field.field_type === 'boolean'}
        <select
          aria-label={field.name}
          name={`property:${field.field_type}:${field.key}`}
          value={String(value(field.key) ?? '')}
          ><option value="">Not set</option
          >{#each field.field_type === 'boolean' ? ['true', 'false'] : field.options as option}<option
              value={option}
              >{option === 'true'
                ? 'Yes'
                : option === 'false'
                  ? 'No'
                  : option.replaceAll('_', ' ')}</option
            >{/each}</select
        >
      {:else}<input
          name={`property:${field.field_type}:${field.key}`}
          type={['number', 'date', 'email', 'url'].includes(field.field_type)
            ? field.field_type
            : 'text'}
          step={field.key === 'employee_count' ? '1' : 'any'}
          min={['employee_count', 'annual_revenue'].includes(field.key)
            ? '0'
            : undefined}
          maxlength="2000"
          value={String(value(field.key) ?? '')}
        />{/if}
    </label>{/each}
</div>
<fieldset>
  <legend>Address</legend>
  <div class="property-grid">
    {#each fields.filter((f) => f.key.startsWith('location.')) as field}<label
        >{field.name}
        {#if field.key === 'location.country_code'}<select
            aria-label={field.name}
            name={`property:text:${field.key}`}
            value={String(value(field.key) ?? '')}
            ><option value="">Select country / region</option
            >{#each countries as country}<option value={country.code}
                >{country.name}</option
              >{/each}</select
          >
        {:else}<input
            name={`property:text:${field.key}`}
            maxlength={field.key === 'location.postal_code' ? 32 : 200}
            value={String(value(field.key) ?? '')}
          />{/if}</label
      >{/each}
  </div>
</fieldset>

<style>
  .property-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0 16px;
  }
  fieldset {
    border: 0;
    border-top: 1px solid var(--border-default);
    margin: 16px 0;
    padding: 16px 0 0;
  }
  legend {
    font-weight: 600;
    padding-right: 12px;
  }
  @media (max-width: 540px) {
    .property-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
