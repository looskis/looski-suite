<script lang="ts">
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
</script>

<svelte:head><title>Confirm email link · Looski Suite</title></svelte:head>
<main>
  <h1>
    {data.kind === 'recovery' ? 'Reset your password' : 'Confirm sign-in'}
  </h1>
  {#if data.kind}
    <p>
      Continue only if you requested this email. This will sign this browser
      into the account that received the link.
    </p>
    <form method="POST"><button>Continue</button></form>
  {:else}<p>
      Open the link in your email to continue. If it has expired, request a new
      one.
    </p>{/if}
  {#if form?.message}<p role="alert">{form.message}</p>{/if}
  <a href="/auth/email">Request a new email link</a>
</main>
