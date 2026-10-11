<script lang="ts">
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
</script>

<svelte:head><title>Contact support</title></svelte:head>
<main class="support">
  <span class="brand">looski · support</span>
  {#if form?.created}<h1>Your ticket is open</h1>
    <p>
      Save this ticket address and private access code. You will need the code
      if you return on another browser or clear your cookies.
    </p>
    <p><a href={form.ticketPath}>Open your ticket →</a></p>
    <label
      >Private access code<input
        readonly
        value={form.code}
        aria-label="Private access code"
      /></label
    >
    <p>
      Anyone with this code can read and reply to this ticket. Keep it private.
      There is no email recovery.
    </p>
  {:else}<h1>How can we help?</h1>
    <p>
      Send a request and follow the conversation here. You will receive a
      private code to return to your ticket.
    </p>
    {#if form?.message}<p role="alert">{form.message}</p>{/if}
    <form method="POST">
      <input type="hidden" name="code" value={form?.code ?? data.code} /><label
        >Your name<input
          name="requester_name"
          required
          maxlength="120"
          autocomplete="name"
          value={form?.input?.requester_name ?? ''}
        /></label
      ><label
        >Subject<input
          name="subject"
          required
          maxlength="200"
          value={form?.input?.subject ?? ''}
        /></label
      ><label
        >How can we help?<textarea
          name="body"
          required
          maxlength="20000"
          rows="7"
          value={form?.input?.body ?? ''}></textarea></label
      ><button>Open ticket</button>
    </form>{/if}
</main>

<style>
  .support {
    overflow-wrap: anywhere;
    max-width: 680px;
    margin: 5vh auto;
    padding: 2rem;
  }
  .brand {
    color: #52705f;
    font-size: 0.9rem;
  }
  input {
    width: 100%;
  }
</style>
