<script lang="ts">
  import RecordDrawer from '$lib/components/ui/RecordDrawer.svelte';
  import type { PageData, ActionData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
</script>

<svelte:head><title>Tags · Looski</title></svelte:head>

<div class="page-heading">
  <h1>Tags</h1>
  <div class="page-actions">
    <RecordDrawer title="New tag" error={Boolean(form?.message)}>
      {#if form?.message}<p role="alert" class="error">{form.message}</p>{/if}
      <form method="POST" action="?/create">
        <input
          type="hidden"
          name="idempotency_key"
          value={form?.key ?? data.key}
        /><label
          >Name<input
            name="name"
            required
            maxlength="80"
            value={form?.input?.name ?? ''}
          /></label
        ><label
          >Color (optional hex)<input
            name="color"
            placeholder="#285b4b"
            value={form?.input?.color ?? ''}
          /></label
        ><button>Create tag</button>
      </form>
    </RecordDrawer>
  </div>
</div>
<section class="index-list">
  <nav class="view-tabs" aria-label="Tag views">
    <a href="?" aria-current={!data.archived ? 'page' : undefined}>All tags</a
    ><a href="?archived=true" aria-current={data.archived ? 'page' : undefined}
      >Archived</a
    >
  </nav>
  <form method="GET" class="inline">
    <label>Find by name<input name="search" value={data.search} /></label><input
      type="hidden"
      name="archived"
      value={String(data.archived)}
    /><button>Search</button>
  </form>
  <div class="record-table">
    <table>
      <thead
        ><tr
          ><th>Tag</th><th>Contacts</th><th>Organizations</th><th
            >Opportunities</th
          ><th>Deals</th></tr
        ></thead
      ><tbody
        >{#each data.page.items as tag}<tr
            ><td
              ><a href={`/workspaces/${data.workspace.id}/tags/${tag.id}`}
                ><span
                  class="tag-swatch"
                  style:background={/^#[0-9a-f]{6}$/i.test(tag.color ?? '')
                    ? tag.color
                    : 'var(--accent)'}
                ></span>{tag.name}</a
              ></td
            ><td
              ><a
                href={`/workspaces/${data.workspace.id}/contacts?tag_id=${tag.id}`}
                >View contacts →</a
              ></td
            ><td
              ><a
                href={`/workspaces/${data.workspace.id}/organizations?tag_id=${tag.id}`}
                >View organizations →</a
              ></td
            ><td
              ><a
                href={`/workspaces/${data.workspace.id}/crm/opportunities?tag_id=${tag.id}`}
                >View opportunities →</a
              ></td
            ><td
              ><a
                href={`/workspaces/${data.workspace.id}/crm/deals?tag_id=${tag.id}`}
                >View deals →</a
              ></td
            ></tr
          >{:else}<tr><td colspan="5">No tags found.</td></tr>{/each}</tbody
      >
    </table>
  </div>
  {#if data.page.next_cursor}<a
      href={`?${new URLSearchParams({ search: data.search, archived: String(data.archived), after: data.page.next_cursor })}`}
      >Next page →</a
    >{/if}
</section>

<style>
  .tag-swatch {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    margin-right: 8px;
  }
</style>
