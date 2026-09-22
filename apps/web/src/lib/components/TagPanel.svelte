<script lang="ts">
  import type { tagPanel } from '$lib/server/tag-panel';
  let {
    panel,
    workspace,
    version,
    draft,
  }: {
    panel: Awaited<ReturnType<typeof tagPanel>>;
    workspace: string;
    version: number;
    draft?: { tagIds: string[]; key: string; expectedVersion: number };
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
      <input
        type="hidden"
        name="idempotency_key"
        value={draft?.key ?? panel.key}
      /><input
        type="hidden"
        name="expected_version"
        value={draft?.expectedVersion ?? version}
      />
      {#each panel.choices as tag}
        <label class="check"
          ><input
            type="checkbox"
            name="tag_id"
            value={tag.id}
            checked={draft
              ? draft.tagIds.includes(tag.id)
              : panel.current.some((t) => t.id === tag.id)}
          />{tag.name}{tag.archived ? ' · Archived' : ''}</label
        >
      {:else}<p>No tags match this search.</p>{/each}
      {#each draft?.tagIds.filter((id) => !panel.choices.some((tag) => tag.id === id)) ?? [] as id}
        <label class="check"
          ><input type="checkbox" name="tag_id" value={id} checked />Submitted
          tag — review before saving</label
        >
      {/each}
      {#if draft}<p>
          <a href="?">Discard tag draft and reload saved values</a>
        </p>{/if}
      <button>Save tags</button>
    </form>
  </section>
{/if}
