<script lang="ts">
  import CmsNav from '$lib/components/CmsNav.svelte';
  let { children } = $props();
</script>

<CmsNav />
{@render children()}
