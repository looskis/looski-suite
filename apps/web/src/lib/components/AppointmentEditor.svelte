<script lang="ts">
  import { untrack } from 'svelte';
  import type { components } from '$lib/api/schema';
  let {
    settings,
    forms,
    fields,
    stages,
  }: {
    settings: components['schemas']['AppointmentSettings'] | null;
    forms: { id: string; name: string }[];
    fields: components['schemas']['Field'][];
    stages: { value: string; name: string }[];
  } = $props();
  const clock = (m: number) =>
    `${Math.floor(m / 60)
      .toString()
      .padStart(2, '0')}:${(m % 60).toString().padStart(2, '0')}`;
  let weekly = $state(
    untrack(() =>
      (
        settings?.weekly ??
        [1, 2, 3, 4, 5].map((weekday) => ({ weekday, start: 540, end: 1020 }))
      ).map((w) => ({ ...w, start: clock(w.start), end: clock(w.end) })),
    ),
  );
  let exceptions = $state(
    untrack(
      () =>
        settings?.exceptions?.flatMap((x) =>
          x.windows.length
            ? x.windows.map((w) => ({
                date: x.date,
                closed: false,
                start: clock(w.start),
                end: clock(w.end),
              }))
            : [{ date: x.date, closed: true, start: '09:00', end: '17:00' }],
        ) ?? [],
    ),
  );
</script>

<label
  >Name<input
    name="name"
    maxlength="120"
    required
    value={settings?.name ?? ''}
  /></label
><label
  >Description<textarea
    name="description"
    maxlength="4000"
    value={settings?.description ?? ''}></textarea></label
>
<div class="inline">
  <label
    >Timezone<input
      name="timezone"
      value={settings?.timezone ?? 'America/Los_Angeles'}
      required
      list="zones"
    /><datalist id="zones"
      ><option>America/Los_Angeles</option><option>America/New_York</option
      ><option>Europe/London</option><option>Europe/Paris</option><option
        >Asia/Singapore</option
      ><option>Australia/Sydney</option><option>UTC</option></datalist
    ></label
  ><label
    >Duration (minutes)<input
      name="duration_minutes"
      type="number"
      min="5"
      max="480"
      value={settings?.duration_minutes ?? 30}
      required
    /></label
  ><label
    >Buffer before<input
      name="buffer_before"
      type="number"
      min="0"
      max="120"
      value={settings?.buffer_before ?? 0}
      required
    /></label
  ><label
    >Buffer after<input
      name="buffer_after"
      type="number"
      min="0"
      max="120"
      value={settings?.buffer_after ?? 0}
      required
    /></label
  >
</div>
<h3>Weekly availability</h3>
<p>
  Windows use this timezone. Buffers must also fit inside availability. Use
  24:00 for midnight at the end of a day. Appointment types you host share one
  calendar.
</p>
<input type="hidden" name="weekly_count" value={weekly.length} />
{#each weekly as w, i}<div class="inline">
    <label
      >Day<select name={`day_${i}`} bind:value={w.weekday}
        >{#each ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'] as day, d}<option
            value={d + 1}>{day}</option
          >{/each}</select
      ></label
    ><label
      >From<input
        type="time"
        name={`start_${i}`}
        bind:value={w.start}
        required
      /></label
    ><label
      >Until<input
        type="text"
        pattern="([01][0-9]|2[0-3]):[0-5][0-9]|24:00"
        name={`end_${i}`}
        bind:value={w.end}
        required
      /></label
    ><button type="button" class="secondary" onclick={() => weekly.splice(i, 1)}
      >Remove window</button
    >
  </div>{/each}<button
  type="button"
  class="secondary"
  onclick={() => weekly.push({ weekday: 1, start: '09:00', end: '17:00' })}
  >Add window</button
>
<h3>Date exceptions</h3>
<p>
  Replace weekly hours on a specific date. Add multiple windows for split hours.
</p>
<input
  type="hidden"
  name="exception_count"
  value={exceptions.length}
/>{#each exceptions as x, i}<div class="inline">
    <label
      >Date<input
        type="date"
        name={`date_${i}`}
        bind:value={x.date}
        required
      /></label
    ><label
      ><input type="checkbox" name={`closed_${i}`} bind:checked={x.closed} /> Closed
      all day</label
    >{#if !x.closed}<label
        >From<input
          type="time"
          name={`exception_start_${i}`}
          bind:value={x.start}
          required
        /></label
      ><label
        >Until<input
          type="text"
          pattern="([01][0-9]|2[0-3]):[0-5][0-9]|24:00"
          name={`exception_end_${i}`}
          bind:value={x.end}
          required
        /></label
      >{/if}<button
      type="button"
      class="secondary"
      onclick={() => exceptions.splice(i, 1)}>Remove exception</button
    >
  </div>{/each}<button
  type="button"
  class="secondary"
  onclick={() =>
    exceptions.push({ date: '', closed: true, start: '09:00', end: '17:00' })}
  >Add exception</button
>
<h3>Intake and reminders</h3>
<label
  >Custom intake form<select name="form_id" value={settings?.form_id ?? ''}
    ><option value="">Name and email only</option>{#each forms as f}<option
        value={f.id}>{f.name}</option
      >{/each}</select
  ></label
>
<p>
  Choose a published, accepting form. Save it first to select its questions for
  CRM follow-up.
</p>
<label
  >Remind attendee (minutes before start)<input
    name="reminder_minutes"
    type="number"
    min="0"
    max="10080"
    value={settings?.reminder_minutes ?? 0}
    required
  /></label
>
<p>
  Use 0 to turn off email reminders. Delivery uses the installation’s email
  settings.
</p>
<details open={Boolean(settings?.crm)}>
  <summary>CRM follow-up</summary><label
    ><input type="checkbox" name="crm" checked={Boolean(settings?.crm)} /> Resolve
    a contact and create an inquiry after booking</label
  ><label
    >Inquiry stage<select
      name="stage"
      value={settings?.crm
        ? `${settings.crm.pipeline_id}/${settings.crm.stage_id}`
        : ''}
      ><option value="">Choose…</option>{#each stages as s}<option
          value={s.value}>{s.name}</option
        >{/each}</select
    ></label
  ><label
    >Inquiry title prefix<input
      name="title_prefix"
      maxlength="120"
      value={settings?.crm?.title_prefix ?? 'Appointment'}
    /></label
  ><label
    >Contact name fields (up to three)<select
      name="name_fields"
      multiple
      value={settings?.crm?.name_fields ?? ['attendee_name']}
      ><option value="attendee_name">Attendee name</option
      >{#each fields.filter((f) => f.kind === 'short_text') as f}<option
          value={f.key}>{f.label}</option
        >{/each}</select
    ></label
  ><label
    >Email field<select
      name="email_field"
      value={settings?.crm?.email_field ?? 'attendee_email'}
      ><option value="">None</option><option value="attendee_email"
        >Attendee email</option
      >{#each fields.filter((f) => f.kind === 'email') as f}<option
          value={f.key}>{f.label}</option
        >{/each}</select
    ></label
  ><label
    >Phone field<select
      name="phone_field"
      value={settings?.crm?.phone_field ?? ''}
      ><option value="">None</option
      >{#each fields.filter((f) => f.kind === 'phone') as f}<option
          value={f.key}>{f.label}</option
        >{/each}</select
    ></label
  ><label
    >Phone country (two-letter code)<input
      name="phone_country"
      maxlength="2"
      value={settings?.crm?.phone_country ?? ''}
    /></label
  ><input
    type="hidden"
    name="owner_membership_id"
    value={settings?.crm?.owner_membership_id ?? ''}
  />
  <p>
    Only these approved fields are used for contact resolution. Existing contact
    details are retained. Ambiguous matches wait for review.
  </p>
</details>
<label
  ><input
    name="accepting"
    type="checkbox"
    checked={settings?.accepting ?? false}
  /> Accept bookings</label
><label
  ><input
    name="archived"
    type="checkbox"
    checked={settings?.archived ?? false}
  /> Archived</label
><button>Save appointment type</button>
