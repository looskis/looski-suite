<script lang="ts">
  import AuthShell from '$lib/components/AuthShell.svelte';
  let { children } = $props();
</script>

<AuthShell>{@render children()}</AuthShell>
