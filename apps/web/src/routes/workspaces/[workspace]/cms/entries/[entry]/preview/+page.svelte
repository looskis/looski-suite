<script lang="ts">
  import PublicContent from '$lib/components/PublicContent.svelte';
  import { page } from '$app/state';
  import type { PageData } from './$types';
  let { data }: { data: PageData } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}/cms`);
</script>

<svelte:head><title>Preview · {data.preview.title} · Looski</title></svelte:head
>
<header>
  <a href={`${root}/entries/${page.params.entry}`}>← Back to editing</a>
</header>
<p role="status">
  Draft preview, visible only to staff who may read drafts. An embedded form
  below is live: answers sent here are real submissions.
</p>
<PublicContent
  page={data.preview}
  mediaUrl={(id) => `${root}/media/${id}/content`}
/>
