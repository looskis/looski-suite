<script lang="ts">
  import type { tagPanel } from '$lib/server/tag-panel';
  let {
    panel,
    workspace,
    version,
  }: {
    panel: Awaited<ReturnType<typeof tagPanel>>;
    workspace: string;
    version: number;
  } = $props();
</script>

{#if panel}
  <section>
    <div class="toolbar">
      <h2>Tags</h2>
      <a href={`/workspaces/${workspace}/tags`}>Manage tags</a>
    </div>
    <form method="GET">
      <label
        >Find a tag<input
          name="tag_search"
          value={panel.search}
          placeholder="Search tag names"
        /></label
      ><button class="secondary">Find tags</button>
    </form>
    {#if panel.more}<p>
        Showing the first 100 matches. Refine your search to find another tag.
      </p>{/if}
    <form method="POST" action="?/tags">
      <input type="hidden" name="idempotency_key" value={panel.key} /><input
        type="hidden"
        name="expected_version"
        value={version}
      />
      {#each panel.choices as tag}
        <label class="check"
          ><input
            type="checkbox"
            name="tag_id"
            value={tag.id}
            checked={panel.current.some((t) => t.id === tag.id)}
          />{tag.name}{tag.archived ? ' · Archived' : ''}</label
        >
      {:else}<p>No tags match this search.</p>{/each}
      <button>Save tags</button>
    </form>
  </section>
{/if}
