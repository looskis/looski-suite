<script lang="ts">
  import type { ActionData } from './$types';
  let { form }: { form: ActionData } = $props();
</script>

<svelte:head><title>Set password · Looski Suite</title></svelte:head>
<main>
  <h1>Set your password</h1>
  <p>Choose a new password with at least 12 characters.</p>
  <form method="POST">
    <label
      >New password <input
        name="password"
        type="password"
        autocomplete="new-password"
        minlength="12"
        maxlength="1024"
        required
      /></label
    >
    <label
      >Confirm password <input
        name="confirm"
        type="password"
        autocomplete="new-password"
        minlength="12"
        maxlength="1024"
        required
      /></label
    >
    <button>Save password</button>
  </form>
  {#if form?.message}<p role="alert">{form.message}</p>{/if}
  <a href="/workspaces">Back to workspaces</a>
</main>
