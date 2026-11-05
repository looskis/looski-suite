<script lang="ts">
  import { page } from '$app/state';
</script>

<svelte:head><title>Form unavailable · Looski</title></svelte:head>
<main>
  <h1>This form is unavailable</h1>
  <p>{page.error?.message}</p>
</main>

<style>
  main {
    max-width: 640px;
    margin: 0 auto;
    padding: var(--space-12) var(--space-5);
  }
</style>
