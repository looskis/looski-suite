<script lang="ts">
  import { page } from '$app/state';
  import FormNav from '$lib/components/FormNav.svelte';
  let { children } = $props();
</script>

<header>
  <a href={`/workspaces/${page.params.workspace}/forms`}>← Forms</a>
</header>
<FormNav />
{@render children()}
