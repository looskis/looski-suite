<script lang="ts">
  import PublicContent from '$lib/components/PublicContent.svelte';
  import type { PageData } from './$types';
  let { data }: { data: PageData } = $props();
</script>

<svelte:head><title>{data.page.title}</title></svelte:head>
<main>
  <PublicContent
    page={data.page}
    mediaUrl={(id) => `/api/v1/public/cms/${data.key}/media/${id}`}
  />
  <p><a href={`/c/${data.key}`}>← All pages</a></p>
</main>

<style>
  main {
    max-width: 720px;
    margin: 0 auto;
    padding: 48px 20px;
  }
</style>
