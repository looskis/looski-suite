<script lang="ts">
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
</script>

<svelte:head><title>Workspace invitation · Looski Suite</title></svelte:head>
<main>
  <h1>Workspace invitation</h1>
  {#if !data.pending}<p>
      Open the private invitation link you received to continue.
    </p>
  {:else if !data.signedIn}
    <p>
      Sign in with the email address that received this invitation. Your
      invitation will remain available after sign-in.
    </p>
    <p>
      <a href="/login">Sign in</a> ·
      <a href="/auth/email">Email sign-in or recovery</a>
    </p>
  {:else if data.preview}
    <h2>Join {data.preview.workspace_name}</h2>
    <p>Invited email: {data.preview.invitation.email}</p>
    <p>
      Roles: {data.preview.invitation.roles.map((role) => role.name).join(', ')}
    </p>
    <p>Expires {data.preview.invitation.expires_at}</p>
    <form method="POST">
      <input
        type="hidden"
        name="idempotency_key"
        value={form?.key ?? data.key}
      />
      <label
        >Your display name <input
          name="display_name"
          maxlength="120"
          required
          value={form?.display_name ?? ''}
        /></label
      >
      <button>Accept invitation</button>
    </form>
  {/if}
  {#if form?.message || data.message}<p role="alert">
      {form?.message ?? data.message}
    </p>{/if}
  {#if data.signedIn}<form method="POST" action="/logout">
      <button>Sign out to use a different account</button>
    </form>{/if}
  <p><a href="/workspaces">Your workspaces</a></p>
</main>

<style>
  :global(body) {
    margin: 0;
    background: #f4f3ee;
    color: #183e35;
    font-family: ui-sans-serif, system-ui, sans-serif;
  }
  main {
    max-width: 520px;
    margin: 64px auto;
    padding: 24px;
  }
  p {
    line-height: 1.6;
  }
  a {
    color: inherit;
  }
  label {
    display: block;
    margin: 24px 0;
  }
  input {
    box-sizing: border-box;
    width: 100%;
    padding: 12px;
    margin-top: 8px;
    border: 1px solid #aabcae;
    border-radius: 8px;
    font: inherit;
  }
  button {
    margin: 12px 0;
    padding: 14px 20px;
    border: 0;
    border-radius: 8px;
    background: #183e35;
    color: white;
    font: inherit;
    cursor: pointer;
  }
  :focus-visible {
    outline: 3px solid #76a990;
    outline-offset: 3px;
  }
</style>
