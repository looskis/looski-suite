<script lang="ts">
  import { untrack } from 'svelte';
  let {
    fields,
  }: {
    fields: {
      name: string;
      archived: string;
      items: { label: string; task_template_revision_id: string }[];
    };
  } = $props();
  let items = $state(untrack(() => fields.items.map((i) => ({ ...i }))));
  $effect(() => {
    items = fields.items.map((i) => ({ ...i }));
  });
</script>

<label
  >Template name<input
    name="name"
    value={fields.name}
    maxlength="120"
    required
  /></label
>
<label
  >Availability<select name="archived" value={fields.archived}
    >{#if !['true', 'false'].includes(fields.archived)}<option
        value={fields.archived}>{fields.archived} (unsupported)</option
      >{/if}<option value="false">Active</option><option value="true"
      >Archived</option
    ></select
  ></label
>
<h3>Ordered checklist</h3>
<p>
  Each item pins a task template revision. Edit the list to publish a new
  revision; existing work remains intact.
</p>
{#each items as item, index}<fieldset>
    <legend>Item {index + 1}</legend><label
      >Label<input
        name="item_label"
        bind:value={item.label}
        maxlength="200"
        required
      /></label
    ><label
      >Task template revision ID<input
        name="task_template_revision_id"
        bind:value={item.task_template_revision_id}
        required
      /></label
    >{#if items.length > 1}<button
        type="button"
        class="secondary"
        onclick={() => items.splice(index, 1)}>Remove item</button
      >{/if}
  </fieldset>{/each}
<button
  type="button"
  class="secondary"
  disabled={items.length >= 25}
  onclick={() => items.push({ label: '', task_template_revision_id: '' })}
  >Add item</button
>
