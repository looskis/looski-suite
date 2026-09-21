<script lang="ts">
  import type { ActionData, PageData } from './$types';
  let { form, data }: { form: ActionData; data: PageData } = $props();
</script>

<svelte:head><title>Sign in · Looski Suite</title></svelte:head>
<main>
  <a class="brand" href="/">LOOSKI SUITE</a>
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
    <p class="help">
      New installation? Ask the person who set up Looski to create your owner
      account.
    </p>
  </section>
</main>

<style>
  :global(body) {
    margin: 0;
    background: #f4f3ee;
    color: #183e35;
    font-family: ui-sans-serif, system-ui, sans-serif;
  }
  main {
    max-width: 880px;
    margin: auto;
    padding: 48px 24px;
  }
  .brand {
    color: inherit;
    text-decoration: none;
    font-size: 12px;
    letter-spacing: 0.2em;
    font-weight: 750;
  }
  section {
    max-width: 400px;
    margin: 80px auto;
  }
  .eyebrow {
    font-size: 11px;
    letter-spacing: 0.15em;
  }
  h1 {
    font-size: 38px;
    letter-spacing: -0.04em;
    font-weight: 500;
    margin-bottom: 12px;
  }
  p {
    color: #526b61;
    line-height: 1.6;
  }
  form {
    margin-top: 32px;
  }
  label {
    display: block;
    font-size: 14px;
    margin: 20px 0 8px;
  }
  input {
    box-sizing: border-box;
    width: 100%;
    padding: 13px;
    border: 1px solid #aabcae;
    border-radius: 8px;
    font: inherit;
    background: #fffefb;
    color: #183e35;
  }
  input:focus-visible,
  button:focus-visible {
    outline: 3px solid #76a990;
    outline-offset: 3px;
  }
  button {
    width: 100%;
    margin-top: 24px;
    border: 0;
    padding: 14px 18px;
    border-radius: 8px;
    color: white;
    background: #183e35;
    font: inherit;
    cursor: pointer;
    display: flex;
    justify-content: space-between;
  }
  .help {
    font-size: 13px;
    margin-top: 28px;
  }
  .error {
    color: #923c26;
  }
</style>
