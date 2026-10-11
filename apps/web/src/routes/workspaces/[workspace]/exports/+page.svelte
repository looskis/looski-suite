<script lang="ts">
  import type { PageData } from './$types';
  let { data }: { data: PageData } = $props();
  let selected = $derived(
    data.page?.manifest.selection.collection ?? 'contacts',
  );
  let parentId = $derived(data.page?.manifest.selection.parent_id ?? '');
  let archived = $derived(
    String(data.page?.manifest.selection.archived ?? false),
  );
  let ended = $derived(String(data.page?.manifest.selection.ended ?? false));
  const spec = $derived(
    data.collections.find((c) => c.collection === selected),
  );
</script>

<svelte:head><title>Export data · Looski Suite</title></svelte:head>
<header>
  <a href="/workspaces">← Workspaces</a><span>{data.workspace.name}</span>
</header>
<h1>Export data</h1>
<p>
  Download the records you can read as JSON. Each download contains up to 25
  records and a manifest. Follow the next page to continue. Live records can
  change between pages.
</p>
<section>
  <form method="GET">
    <input type="hidden" name="run" value="true" />
    <label
      >Collection<select name="collection" bind:value={selected}
        >{#each data.collections as collection}<option
            value={collection.collection}
            >{collection.collection.replaceAll('_', ' ')}</option
          >{/each}</select
      ></label
    >
    {#if spec?.parent_required}<label
        >Parent record ID<input
          name="parent_id"
          bind:value={parentId}
          required
          placeholder="UUID of the contact, deal, task, template or form"
        /></label
      >{/if}
    {#if spec?.supports_archive_selection}<label
        >Records<select name="archived" bind:value={archived}
          ><option value="false">Active</option><option value="true"
            >Archived</option
          ></select
        ></label
      >{/if}
    {#if spec?.supports_ended_selection}<label
        >Relationships<select name="ended" bind:value={ended}
          ><option value="false">Current</option><option value="true"
            >Ended</option
          ></select
        ></label
      >{/if}
    <button>Preview export page</button>
  </form>
</section>
{#if data.page}
  <section>
    <h2>{data.page.items.length} records on this page</h2>
    <p>
      {data.page.manifest.selection.collection.replaceAll('_', ' ')} · {data
        .page.manifest.selection.archived
        ? 'Archived'
        : 'Current selection'}
    </p>
    {#if data.download}<a class="button" href={data.download}
        >Download this page</a
      >{/if}
    {#if data.next}<a class="button" href={data.next}>Next page →</a>{:else}<p>
        End of this collection and selection.
      </p>{/if}
    <details>
      <summary>View JSON</summary>
      <pre>{JSON.stringify(data.page, null, 2)}</pre>
    </details>
  </section>
{/if}
<p class="muted">
  Exports include the selected business records. Credentials, private invitation
  links, diagnostic logs and recovery data are excluded. This download is not an
  installation backup.
</p>

<style>
  pre {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    max-height: 32rem;
    overflow: auto;
  }
  .button {
    display: inline-block;
    margin: 12px 12px 12px 0;
  }
</style>
