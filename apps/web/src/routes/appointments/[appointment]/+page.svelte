<script lang="ts">
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const when = (s: string, zone: string) =>
    new Intl.DateTimeFormat('en', {
      timeZone: zone,
      dateStyle: 'full',
      timeStyle: 'long',
    }).format(new Date(s));
</script>

<svelte:head><title>Manage appointment · Looski</title></svelte:head>
<main>
  {#if form?.message}<p role="alert">{form.message}</p>{/if}
  {#if data.booking}<h1>{data.booking.name}</h1>
    <p>{when(data.booking.starts_at, data.booking.timezone)}</p>
    <p>{data.booking.state}</p>
    {#if data.booking.state === 'confirmed'}<section>
        <h2>Choose another time</h2>
        <form method="GET" class="inline">
          <label
            >Day<input
              type="date"
              name="date"
              value={data.date}
              required
            /></label
          ><button class="secondary">Show times</button>
        </form>
        {#if data.slots.length}<form method="POST" action="?/change">
            <input
              type="hidden"
              name="idempotency_key"
              value={data.key}
            /><input
              type="hidden"
              name="expected_version"
              value={data.booking.version}
            /><input type="hidden" name="action" value="reschedule" /><label
              >New time<select name="starts_at" required
                >{#each data.slots as s}<option value={s.starts_at}
                    >{s.label}</option
                  >{/each}</select
              ></label
            ><button>Reschedule appointment</button>
          </form>{:else}<p>
            No times available. Choose another day or contact your host.
          </p>{/if}
      </section>
      <section>
        <h2>Cancel appointment</h2>
        <p>This releases your reserved time.</p>
        <form method="POST" action="?/change">
          <input
            type="hidden"
            name="idempotency_key"
            value={`${data.key}:cancel`}
          /><input
            type="hidden"
            name="expected_version"
            value={data.booking.version}
          /><button name="action" value="cancel" class="secondary"
            >Cancel appointment</button
          >
        </form>
      </section>{/if}
    <form method="POST" action="?/lock">
      <button class="secondary">Lock this page</button>
    </form>
  {:else}<h1>Manage your appointment</h1>
    <form method="POST" action="?/unlock">
      <label
        >Private access code<input
          name="code"
          type="password"
          required
          autocomplete="off"
        /></label
      ><button>Open appointment</button>
    </form>{/if}
</main>

<style>
  main {
    max-width: 680px;
    margin: auto;
    padding: 3rem 1.25rem;
    overflow-wrap: anywhere;
  }
  section {
    margin: 2rem 0;
  }
</style>
