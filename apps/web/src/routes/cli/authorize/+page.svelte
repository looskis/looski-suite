<script lang="ts">
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
</script>

<svelte:head><title>Authorize CLI · Looski Suite</title></svelte:head>
<main>
  <p class="eyebrow">COMMAND LINE ACCESS</p>
  {#if form && 'code' in form && form.code}
    <h1>Copy this code</h1>
    <p>Paste it into the terminal that is waiting for it.</p>
    <p>
      <output class="code" aria-label="Authorization code">{form.code}</output>
    </p>
    <p class="help">
      It works once and expires in {Math.round(form.expiresIn / 60)} minutes. Never
      share it; it signs in as you.
    </p>
  {:else if form && 'cancelled' in form}
    <h1>Sign-in cancelled</h1>
    <p>No access was granted. You can close this page.</p>
  {:else}
    <h1>Authorize the Looski CLI?</h1>
    <p>
      This lets the command line and local MCP tools on a computer act as
      {data.email ?? 'the account signed in to this browser'} on
      <strong>{data.origin}</strong>, with your current workspace permissions.
    </p>
    {#if data.device}<p>
        Requesting device (as reported by it): <strong>{data.device}</strong>
      </p>{/if}
    <p class="help">
      Only continue if you just ran <code>looski login</code> yourself. If someone
      sent you this link, cancel.
    </p>
    <form method="POST">
      <input type="hidden" name="code_challenge" value={data.challenge} />
      <input type="hidden" name="code_challenge_method" value="S256" />
      <input type="hidden" name="state" value={data.state} />
      {#if data.device}<input
          type="hidden"
          name="device"
          value={data.device}
        />{/if}
      <button formaction={`?/approve&${data.query}`}>Approve</button>
      <button formaction={`?/cancel&${data.query}`} class="secondary"
        >Cancel</button
      >
    </form>
  {/if}
  {#if form && 'message' in form && form.message}<p class="error" role="alert">
      {form.message}
    </p>{/if}
</main>

<style>
  .code {
    display: block;
    font-family: ui-monospace, monospace;
    font-size: 14px;
    padding: 12px;
    border: 1px solid var(--line);
    border-radius: 8px;
    word-break: break-all;
    user-select: all;
  }
</style>
