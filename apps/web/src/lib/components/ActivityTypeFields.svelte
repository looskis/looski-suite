<script lang="ts">
  let {
    fields,
  }: {
    fields: {
      name: string;
      behavior: string;
      position: string | number;
      archived: string | boolean;
    };
  } = $props();
</script>

<label
  >Name<input name="name" value={fields.name} maxlength="120" required /></label
>
<label
  >Behavior<select name="behavior" value={fields.behavior}
    >{#if !['note', 'interaction'].includes(fields.behavior)}<option
        value={fields.behavior}>{fields.behavior} (unsupported)</option
      >{/if}<option value="note">Note</option><option value="interaction"
      >Logged interaction</option
    ></select
  ></label
>
<p class="muted">
  Behavior is fixed after creation. Create a new type to change its meaning.
</p>
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
