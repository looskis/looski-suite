<script lang="ts">
  import type { ActionData, PageData } from './$types';
  import BookingIntake from '$lib/components/BookingIntake.svelte';
  let { data, form }: { data: PageData; form: ActionData } = $props();
</script>

<svelte:head><title>{data.type.name} · Book an appointment</title></svelte:head>
<main>
  {#if form?.created}<h1>Your appointment is confirmed</h1>
    <p>
      Save this private access code. Anyone with it can view, reschedule, or
      cancel your appointment.
    </p>
    <code class="secret">{form.code}</code>
    <p><a href={form.bookingPath}>Manage your appointment →</a></p>
  {:else}<h1>{data.type.name}</h1>
    <p>{data.type.description}</p>
    <p>{data.type.duration_minutes} minutes · {data.type.timezone}</p>
    {#if form?.message}<p role="alert">{form.message}</p>{/if}
    <form method="GET" class="inline">
      <label
        >Choose a day<input
          type="date"
          name="date"
          value={data.date}
          required
        /></label
      ><button class="secondary">Show times</button>
    </form>
    {#if data.slots.length}<form method="POST">
        <input
          type="hidden"
          name="code"
          value={form?.code ?? data.code}
        /><input
          type="hidden"
          name="form_version_id"
          value={data.type.form?.id ?? ''}
        /><label
          >Available time<select
            name="starts_at"
            value={form?.values?.starts_at ?? ''}
            required
            ><option value="">Choose…</option>{#each data.slots as s}<option
                value={s.starts_at}>{s.label}</option
              >{/each}</select
          ></label
        >
        <label
          >Your name<input
            name="attendee_name"
            maxlength="200"
            required
            value={form?.values?.attendee_name ?? ''}
          /></label
        ><label
          >Email<input
            name="attendee_email"
            type="email"
            maxlength="254"
            required
            value={form?.values?.attendee_email ?? ''}
          /></label
        >
        {#if data.type.form}<h2>{data.type.form.name}</h2>
          <BookingIntake
            definition={data.type.form.definition}
            input={form?.input}
            errors={form?.errors}
          />{/if}
        <button>Confirm appointment</button>
        <p>Your time is reserved only after confirmation.</p>
      </form>{:else}<p>
        No times are available on this day. Choose another date.
      </p>{/if}{/if}
  <p>Powered by Looski Suite</p>
</main>

<style>
  main {
    max-width: 680px;
    margin: auto;
    padding: 3rem 1.25rem;
    overflow-wrap: anywhere;
  }
  .secret {
    display: block;
    padding: 1rem;
    background: var(--surface, #f5f5f5);
    overflow-wrap: anywhere;
  }
</style>
