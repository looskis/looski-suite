<script lang="ts">
  let { fields, creating = false } = $props<{
    creating?: boolean;
    fields: {
      title: string;
      notes: string;
      assigned_membership_id: string;
      due_on: string;
      status: string;
    };
  }>();
</script>

<label
  >Title<input
    name="title"
    value={fields.title}
    required
    maxlength="200"
  /></label
>
<label
  >Notes<textarea name="notes" maxlength="10000">{fields.notes}</textarea
  ></label
>
<label
  >Assignee membership ID<input
    name="assigned_membership_id"
    value={fields.assigned_membership_id}
    placeholder={creating
      ? 'Leave blank to assign to yourself'
      : 'Leave blank to keep the current assignee'}
  /></label
>
<label>Due date<input type="date" name="due_on" value={fields.due_on} /></label>
<label
  >Status<select name="status" value={fields.status}>
    {#if !['open', 'in_progress', 'completed', 'cancelled'].includes(fields.status)}<option
        value={fields.status}>Unsupported status: {fields.status}</option
      >{/if}
    <option value="open">Open</option><option value="in_progress"
      >In progress</option
    ><option value="completed">Completed</option><option value="cancelled"
      >Cancelled</option
    >
  </select></label
>
