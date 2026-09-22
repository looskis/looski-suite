<script lang="ts">
  import WorkspaceShell from '$lib/components/WorkspaceShell.svelte';
  let { children } = $props();
</script>

<WorkspaceShell>{@render children()}</WorkspaceShell>
