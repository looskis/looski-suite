<script lang="ts">
  import type { PageData } from './$types';
  let { data }: { data: PageData } = $props();
</script>

<svelte:head><title>{data.index.name}</title></svelte:head>
<main>
  <h1>{data.index.name}</h1>
  {#if data.index.description}<p class="description">
      {data.index.description}
    </p>{/if}
  <ul>
    {#each data.index.entries as entry}
      <li><a href={`/c/${data.key}/${entry.slug}`}>{entry.title}</a></li>
    {:else}
      <li>Nothing has been published yet.</li>
    {/each}
  </ul>
  {#if data.index.next_cursor}
    <a href={`?after=${encodeURIComponent(data.index.next_cursor)}`}
      >Next page →</a
    >
  {/if}
</main>

<style>
  main {
    max-width: 720px;
    margin: 0 auto;
    padding: 48px 20px;
    overflow-wrap: anywhere;
  }
  .description {
    white-space: pre-wrap;
  }
</style>
