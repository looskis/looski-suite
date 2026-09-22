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
