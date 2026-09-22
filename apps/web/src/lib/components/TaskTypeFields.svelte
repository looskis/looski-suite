<script lang="ts">
  let {
    fields,
  }: {
    fields: {
      name: string;
      position: string | number;
      archived: string | boolean;
    };
  } = $props();
</script>

<label
  >Name<input name="name" value={fields.name} maxlength="120" required /></label
>
<label
  >Order<input
    type="number"
    name="position"
    step="1"
    value={fields.position}
  /></label
>
<label
  >Availability<select name="archived" value={String(fields.archived)}
    >{#if !['true', 'false'].includes(String(fields.archived))}<option
        value={String(fields.archived)}
        >{String(fields.archived)} (unsupported)</option
      >{/if}<option value="false">Active</option><option value="true"
      >Archived</option
    ></select
  ></label
>
