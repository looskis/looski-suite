<script lang="ts">
  import Icon from '$lib/components/Icon.svelte';
  import AuthShell from '$lib/components/AuthShell.svelte';
  import type { ActionData, PageData } from './$types';
  let showPassword = $state(false);
  let { form, data }: { form: ActionData; data: PageData } = $props();
</script>

<svelte:head
  ><title>Sign in · Looski Suite</title><meta
    name="robots"
    content="noindex"
  /></svelte:head
>
<AuthShell>
  <main>
    <section>
      <p class="eyebrow">LOOSKI SUITE</p>
      <h1>Welcome back.</h1>
      <p>
        {data.cliAuthorization
          ? 'Sign in to continue authorizing the Looski CLI.'
          : 'Good to see you. Let’s get back to your business.'}
      </p>
      {#if data.localLogout}<p role="status">
          You are signed out of this browser. The service could not confirm
          session revocation.
        </p>{/if}
      {#if data.googleFailed}
        <p class="error" role="alert">
          Google sign-in could not be completed. Try again with the Google email
          address associated with your Looski account, or use your password.
        </p>
      {/if}
      {#if data.googleEnabled}
        <form method="POST" action={data.googleAction}>
          <button type="submit" class="secondary">Continue with Google</button>
        </form>
        <p class="sign-in-divider">or sign in with email</p>
      {/if}
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
        <div class="password-field">
          <input
            id="password"
            name="password"
            type={showPassword ? 'text' : 'password'}
            autocomplete="current-password"
            maxlength="1024"
            required
          /><button
            class="password-toggle"
            type="button"
            aria-controls="password"
            aria-label={showPassword ? 'Hide password' : 'Show password'}
            aria-pressed={showPassword}
            onclick={() => (showPassword = !showPassword)}
            >{showPassword ? 'Hide' : 'Show'}</button
          >
        </div>
        {#if form?.message}<p class="error" role="alert">{form.message}</p>{/if}
        <button type="submit">Sign in <Icon name="arrow" size={17} /></button>
      </form>
      <p><a href="/auth/email">Email sign-in or password recovery</a></p>
      <p class="help">
        Joining a team? Ask your workspace administrator for an invitation.
        Setting up your own installation? Use the owner setup link from your
        installer.
      </p>
    </section>
  </main>
</AuthShell>

<style>
  .password-field {
    position: relative;
  }
  .password-field input {
    padding-right: 68px;
  }
  .password-field .password-toggle {
    position: absolute;
    right: 4px;
    top: 4px;
    min-height: 40px;
    width: 56px;
    margin: 0;
    padding: 8px;
    background: transparent;
    color: var(--brand-muted);
    border: 0;
    font-size: 11px;
    font-weight: 500;
    box-shadow: none;
  }
  .password-field .password-toggle:hover {
    color: var(--brand-ink);
    background: var(--surface-hover);
  }
  .sign-in-divider {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 11px;
    margin: 23px 0 0;
  }
  .sign-in-divider::before,
  .sign-in-divider::after {
    content: '';
    flex: 1;
    border-top: 1px solid var(--brand-line);
  }
</style>
