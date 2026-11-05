<script lang="ts">
  import { page } from '$app/state';
</script>

<svelte:head><title>Page unavailable</title></svelte:head>
<main>
  <h1>This page is unavailable</h1>
  <p>{page.error?.message}</p>
</main>

<style>
  main {
    max-width: 720px;
    margin: 0 auto;
    padding: var(--space-12) var(--space-5);
  }
</style>
