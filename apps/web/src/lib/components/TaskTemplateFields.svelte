<script lang="ts">
  let {
    fields,
  }: {
    fields: {
      name: string;
      archived: string;
      title: string;
      notes: string;
      type_id: string;
      assignment: string;
      fixed_membership_id: string;
      due_offset_days: string;
    };
  } = $props();
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
  >Task title<input
    name="title"
    value={fields.title}
    maxlength="200"
    required
  /></label
>
<label
  >Task notes<textarea name="notes" maxlength="10000" rows="4"
    >{fields.notes}</textarea
  ></label
>
<label
  >Task type ID (optional)<input name="type_id" value={fields.type_id} /></label
>
<label
  >Assignment<select name="assignment" value={fields.assignment}
    >{#if !['initiator', 'context_owner', 'fixed'].includes(fields.assignment)}<option
        value={fields.assignment}>{fields.assignment} (unsupported)</option
      >{/if}<option value="initiator">Initiating staff member</option><option
      value="context_owner">Contextual record owner</option
    ><option value="fixed">Fixed staff membership</option></select
  ></label
>
<label
  >Fixed membership ID (fixed assignment only)<input
    name="fixed_membership_id"
    value={fields.fixed_membership_id}
  /></label
>
<label
  >Due date offset in calendar days<input
    type="number"
    name="due_offset_days"
    value={fields.due_offset_days}
    min="-3650"
    max="3650"
    step="1"
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
