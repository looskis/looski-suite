<script lang="ts">
  import { page } from '$app/state';
  import ProjectNav from '$lib/components/ProjectNav.svelte';
  let { children } = $props();
</script>

<header>
  <a href={`/workspaces/${page.params.workspace}/projects`}>← Projects</a>
</header>
<ProjectNav />
{@render children()}
