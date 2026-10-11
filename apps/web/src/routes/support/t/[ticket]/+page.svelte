<script lang="ts">
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
</script>

<svelte:head
  ><title>{data.ticket?.subject ?? 'Your support ticket'}</title></svelte:head
>
<main class="support">
  <span>looski · support</span>
  {#if form?.message}<p role="alert">{form.message}</p>{/if}
  {#if data.ticket && data.messages}<h1>{data.ticket.subject}</h1>
    <p>Status: <strong>{data.ticket.status}</strong></p>
    <ol>
      {#each data.messages.items as message}<li>
          <strong>{message.kind === 'customer' ? 'You' : 'Support'}</strong
          ><small>{message.created_at}</small>
          <p>{message.body}</p>
        </li>{/each}
    </ol>
    {#if data.messages.next_cursor}<a
        href={`?after=${data.messages.next_cursor}`}>More messages →</a
      >{/if}
    <form method="POST" action="?/reply">
      <input
        type="hidden"
        name="idempotency_key"
        value={form?.key ?? data.key}
      /><label
        >Your reply<textarea
          name="body"
          rows="5"
          maxlength="20000"
          required
          value={form?.body ?? ''}></textarea></label
      >{#if data.ticket.status === 'resolved'}<p>
          Sending a reply will reopen your ticket.
        </p>{/if}<button>Send reply</button>
    </form>
    <form method="POST" action="?/lock">
      <button class="secondary">Forget access on this browser</button>
    </form>
  {:else}<h1>Your support ticket</h1>
    <p>Enter the private access code you saved when opening this ticket.</p>
    <form method="POST" action="?/unlock">
      <label
        >Access code<input
          name="code"
          type="password"
          required
          autocomplete="off"
          minlength="64"
          maxlength="64"
        /></label
      ><button>Open ticket</button>
    </form>{/if}
</main>

<style>
  .support {
    overflow-wrap: anywhere;
    max-width: 720px;
    margin: 5vh auto;
    padding: 2rem;
  }
  ol {
    list-style: none;
    padding: 0;
  }
  li {
    padding: 1rem 0;
    border-bottom: 1px solid #dce5de;
  }
  li p {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  small {
    display: block;
    color: #68786e;
  }
  form {
    margin-top: 1.5rem;
  }
</style>
