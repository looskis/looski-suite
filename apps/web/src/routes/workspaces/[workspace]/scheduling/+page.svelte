<script lang="ts">
  import type { ActionData, PageData } from './$types';
  import AppointmentEditor from '$lib/components/AppointmentEditor.svelte';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}/scheduling`);
</script>

<svelte:head><title>Scheduling · Looski</title></svelte:head>
<header><span>{data.workspace.name}</span></header>
<h1>Scheduling</h1>
<p>Share bookable time, collect intake, and keep follow-up together.</p>
{#if form?.message}<p role="alert">{form.message}</p>{/if}
{#if data.types}<section>
    <div class="toolbar">
      <h2>Appointment types</h2>
      <a href={data.archived ? '?' : '?archived=true'}
        >{data.archived ? 'Show active' : 'Show archived / cancelled'}</a
      >
    </div>
    {#each data.types.items as t}<article>
        <h3><a href={`${root}?type=${t.id}`}>{t.settings.name}</a></h3>
        <p>
          {t.settings.duration_minutes} minutes · {t.settings.timezone} · {t
            .settings.accepting
            ? 'Accepting bookings'
            : 'Closed'}
        </p>
        {#if t.settings.accepting && !t.settings.archived}<a
            href={`${data.origin}/book/${t.public_key}`}
            >{data.origin}/book/{t.public_key}</a
          >{/if}
      </article>{:else}<p>
        Create an appointment type to share your first booking page.
      </p>{/each}
  </section>
  <section>
    <div class="toolbar">
      <h2>{data.editing ? 'Edit appointment type' : 'New appointment type'}</h2>
      {#if data.editing}<a href={root}>Create another</a>{/if}
    </div>
    <form method="POST" action="?/save">
      <input type="hidden" name="idempotency_key" value={data.key} /><input
        type="hidden"
        name="id"
        value={data.editing?.id ?? ''}
      /><input
        type="hidden"
        name="expected_version"
        value={data.editing?.version ?? ''}
      />{#key JSON.stringify([data.editing?.id, form?.draft])}<AppointmentEditor
          settings={form?.draft ?? data.editing?.settings ?? null}
          forms={data.forms}
          fields={data.fields}
          stages={data.stages}
        />{/key}
    </form>
  </section>
  <section>
    <h2>{data.archived ? 'Cancelled appointments' : 'Appointments'}</h2>
    <div class="table-wrap">
      <table>
        <thead
          ><tr
            ><th>Appointment</th><th>Attendee</th><th>Starts (UTC)</th><th
              >Follow-up</th
            ></tr
          ></thead
        ><tbody
          >{#each data.appointments?.items ?? [] as b}<tr
              ><td><a href={`${root}/${b.id}`}>{b.name}</a></td><td
                >{b.attendee_name}</td
              ><td>{b.starts_at}</td><td
                >{b.job_state === 'blocked' || b.job_state === 'exhausted'
                  ? `${b.job_state}: ${b.error_code}`
                  : b.recipe_state}</td
              ></tr
            >{:else}<tr
              ><td colspan="4">No appointments in this selection.</td></tr
            >{/each}</tbody
        >
      </table>
    </div>
    {#if data.appointments?.next_cursor}<a
        href={`?after=${data.appointments.next_cursor}&archived=${data.archived}`}
        >Next page →</a
      >{/if}
  </section>
{:else}<section>
    <h2>Scheduling is turned off</h2>
    <p>Appointments are retained. Public access and pending work are paused.</p>
  </section>{/if}
{#if data.module}<section>
    <h2>Application selection</h2>
    <form method="POST" action="?/module">
      <input type="hidden" name="idempotency_key" value={data.key} /><input
        type="hidden"
        name="expected_version"
        value={data.module.version}
      /><input
        type="hidden"
        name="enabled"
        value={String(!data.module.enabled)}
      /><button class="secondary"
        >{data.module.enabled ? 'Turn off' : 'Turn on'} Scheduling</button
      >
    </form>
  </section>{/if}

<style>
  article {
    padding: 1rem 0;
    border-bottom: 1px solid var(--line, #ddd);
    overflow-wrap: anywhere;
  }
</style>
