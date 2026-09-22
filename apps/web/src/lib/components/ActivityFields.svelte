<script lang="ts">
  import type { components } from '$lib/api/schema';
  let {
    fields,
    types = [],
  }: {
    fields: {
      type_id: string;
      subject: string;
      body: string;
      occurred_at: string;
      duration_minutes: string;
      responsible_membership_id: string;
    };
    types?: components['schemas']['ActivityType'][];
  } = $props();
</script>

<label
  >Activity type ID<input
    name="type_id"
    value={fields.type_id}
    list="activity-types"
    required
  /></label
>
<datalist id="activity-types"
  >{#each types as type}<option value={type.id}
      >{type.name} · {type.behavior}</option
    >{/each}</datalist
>
<label
  >Subject<input
    name="subject"
    value={fields.subject}
    maxlength="200"
    required
  /></label
>
<label
  >Details<textarea name="body" rows="5" maxlength="10000"
    >{fields.body}</textarea
  ></label
>
<label
  >Occurred at<input
    name="occurred_at"
    value={fields.occurred_at}
    placeholder="2026-10-12T09:00:00-07:00"
    required
  /></label
>
<p class="muted">Include the timezone offset or Z for UTC.</p>
<label
  >Duration in minutes (interactions only)<input
    name="duration_minutes"
    type="number"
    min="0"
    max="10080"
    step="1"
    value={fields.duration_minutes}
  /></label
>
<label
  >Responsible membership ID<input
    name="responsible_membership_id"
    value={fields.responsible_membership_id}
  /></label
>
<p class="muted">
  Leave responsibility blank to keep the current staff member, or assign
  yourself on creation.
</p>
