<script lang="ts">
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
</script>

<svelte:head><title>Email sign-in · Looski Suite</title></svelte:head>
<main>
  <a href="/login">Back to sign in</a>
  <h1>Sign in by email</h1>
  {#if data.enabled}
    <p>
      Use the email address associated with your account. Links expire after one
      hour.
    </p>
    <form method="POST">
      <label
        >Email <input
          type="email"
          name="email"
          autocomplete="username"
          maxlength="254"
          required
        /></label
      >
      <button name="kind" value="magiclink">Email me a sign-in link</button>
      <button name="kind" value="recovery">Reset my password</button>
    </form>
  {:else}<p>
      Email sign-in is not configured for this installation. Ask your
      administrator to configure outgoing mail.
    </p>{/if}
  {#if form?.message}<p role="status">{form.message}</p>{/if}
</main>
