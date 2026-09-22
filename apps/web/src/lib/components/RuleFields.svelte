<script lang="ts">
  let {
    fields,
  }: {
    fields: {
      name: string;
      status_id: string;
      revision_id: string;
      archived: string;
    };
  } = $props();
</script>

<label
  >Rule name<input
    name="name"
    value={fields.name}
    maxlength="120"
    required
  /></label
>
<label
  >Deal status ID<input
    name="status_id"
    value={fields.status_id}
    required
  /></label
>
<label
  >Checklist revision ID<input
    name="revision_id"
    value={fields.revision_id}
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
