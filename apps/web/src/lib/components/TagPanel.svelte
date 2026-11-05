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
    <div class="tag-chips">
      {#each panel.current as tag}<span class="tag-chip"
          >{tag.name}{tag.archived ? ' · Archived' : ''}</span
        >{:else}<span class="muted">No tags assigned.</span>{/each}
    </div>
    <details open={Boolean(draft || panel.search)}>
      <summary>Edit tags</summary>
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
        <div class="tag-choices">
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
              ><input
                type="checkbox"
                name="tag_id"
                value={id}
                checked
              />Submitted tag — review before saving</label
            >
          {/each}
        </div>
        {#if draft}<p>
            <a href="?">Discard tag draft and reload saved values</a>
          </p>{/if}
        <button>Save tags</button>
      </form>
    </details>
  </section>
{/if}

<style>
  .tag-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 8px 0 16px;
    font-size: 0.75rem;
  }
  .tag-chip {
    display: inline-block;
    padding: 3px 8px;
    border: 1px solid var(--border-default);
    border-radius: 5px;
    background: var(--surface-canvas);
  }
  .tag-choices {
    max-height: 300px;
    overflow-y: auto;
    margin: 16px 0;
    padding-right: 6px;
  }
  .tag-choices label {
    margin: 10px 0;
    font-size: 0.75rem;
  }
</style>
