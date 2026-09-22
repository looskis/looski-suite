<script lang="ts">
  import AuthShell from '$lib/components/AuthShell.svelte';
  import type { ActionData, PageData } from './$types';
  let { form, data }: { form: ActionData; data: PageData } = $props();
</script>

<svelte:head><title>Sign in · Looski Suite</title></svelte:head>
<AuthShell>
  <main>
    <section>
      <p class="eyebrow">WELCOME BACK</p>
      <h1>Your business starts here.</h1>
      <p>Sign in to your workspace.</p>
      {#if data.localLogout}<p role="status">
          You are signed out of this browser. The service could not confirm
          session revocation.
        </p>{/if}
      <form method="POST">
        <label for="email">Email</label>
        <input
          id="email"
          name="email"
          type="email"
          autocomplete="username"
          maxlength="254"
          required
        />
        <label for="password">Password</label>
        <input
          id="password"
          name="password"
          type="password"
          autocomplete="current-password"
          maxlength="1024"
          required
        />
        {#if form?.message}<p class="error" role="alert">{form.message}</p>{/if}
        <button type="submit">Sign in <span aria-hidden="true">→</span></button>
      </form>
      <p><a href="/auth/email">Email sign-in or password recovery</a></p>
      <p class="help">
        New installation? Ask the person who set up Looski to create your owner
        account.
      </p>
    </section>
  </main>
</AuthShell>
