<script lang="ts">
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const b = $derived(data.booking);
  const root = $derived(`/workspaces/${data.workspace.id}`);
</script>

<svelte:head><title>{b.name} · Scheduling</title></svelte:head><a
  href={`${root}/scheduling`}>← Scheduling</a
>
<h1>{b.name}</h1>
{#if form?.message}<p role="alert">{form.message}</p>{/if}
<p>{b.attendee_name} · {b.attendee_email}</p>
<p>{b.starts_at} – {b.ends_at} (UTC) · {b.timezone} · {b.state}</p>
{#if b.state === 'confirmed'}<section>
    <h2>Change reservation</h2>
    <form method="POST" action="?/change">
      <input type="hidden" name="idempotency_key" value={data.key} /><input
        type="hidden"
        name="expected_version"
        value={b.version}
      /><label
        >New start (UTC, every 15 minutes)<input
          name="starts_at"
          type="datetime-local"
          step="900"
          required
        /></label
      ><input type="hidden" name="action" value="reschedule" /><button
        >Reschedule</button
      >
    </form>
    <form method="POST" action="?/change">
      <input
        type="hidden"
        name="idempotency_key"
        value={`${data.key}:cancel`}
      /><input type="hidden" name="expected_version" value={b.version} /><button
        class="secondary"
        name="action"
        value="cancel">Cancel appointment</button
      >
    </form>
  </section>{/if}
<section>
  <h2>Intake</h2>
  {#if b.intake}<p>Form version {b.intake.version_number}</p>
    <dl>
      {#each Object.entries(b.intake.answers) as [key, value]}<dt>{key}</dt>
        <dd>
          {typeof value === 'string' ? value : JSON.stringify(value)}
        </dd>{/each}
    </dl>{:else}<p>
      No custom intake available under your current access.
    </p>{/if}
</section>
<section>
  <h2>CRM follow-up</h2>
  <p>{b.recipe_state}{b.job_state ? ` · ${b.job_state}` : ''}</p>
  {#if b.error_code}<p role="status">
      {b.error_code}. The booking is retained; resolve the cause and retry
      follow-up.
    </p>{/if}{#if b.contact_id}<p>
      <a
        href={`${root}/crm/timeline?anchor_kind=contact&anchor_id=${b.contact_id}`}
        >Contact timeline</a
      >
    </p>{/if}{#if b.intake_id}<p>
      <a href={`${root}/intakes/${b.intake_id}`}>Review contact resolution</a>
    </p>{/if}{#if b.recipe_job_id}<p>
      <a href={`${root}/jobs/${b.recipe_job_id}`}>Job history</a>
    </p>{/if}{#if b.recipe_state === 'awaiting_review' || b.job_state === 'blocked' || b.job_state === 'exhausted'}<form
      method="POST"
      action="?/retry"
    >
      <input
        type="hidden"
        name="idempotency_key"
        value={`${data.key}:retry`}
      /><input type="hidden" name="expected_version" value={b.version} /><button
        >Retry follow-up</button
      >
    </form>{/if}
</section>
<section>
  <h2>Reservation history</h2>
  {#each data.events.toSorted((a, b) => a.version - b.version) as e}<p>
      #{e.version} · {e.action} · {e.starts_at} – {e.ends_at}
    </p>{/each}
</section>
