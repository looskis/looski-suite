<script lang="ts">
  import type { PageData, ActionData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
</script>

<svelte:head><title>Tags · Looski</title></svelte:head>
<header>
  <a href={`/workspaces/${data.workspace.id}/contacts`}>Contacts</a><a
    href={`/workspaces/${data.workspace.id}/organizations`}>Organizations</a
  ><span>{data.workspace.name}</span>
</header>
<h1>Tags for your workspace.</h1>
<p>
  Use shared labels to organize records. Renaming a tag keeps its assignments.
</p>
<section>
  <h2>{data.archived ? 'Archived tags' : 'Tags'}</h2>
  <a href={data.archived ? '?' : '?archived=true'}
    >{data.archived ? 'Show active' : 'Show archived'}</a
  >
  <form method="GET">
    <label>Find by name<input name="search" value={data.search} /></label><input
      type="hidden"
      name="archived"
      value={String(data.archived)}
    /><button>Search</button>
  </form>
  <ul>
    {#each data.page.items as tag}<li>
        <a href={`/workspaces/${data.workspace.id}/tags/${tag.id}`}
          >{tag.name}</a
        >
        ·
        <a href={`/workspaces/${data.workspace.id}/contacts?tag_id=${tag.id}`}
          >Contacts</a
        >
        ·
        <a
          href={`/workspaces/${data.workspace.id}/organizations?tag_id=${tag.id}`}
          >Organizations</a
        >
        ·
        <a
          href={`/workspaces/${data.workspace.id}/crm/opportunities?tag_id=${tag.id}`}
          >Opportunities</a
        >
        ·
        <a href={`/workspaces/${data.workspace.id}/crm/deals?tag_id=${tag.id}`}
          >Deals</a
        >
      </li>{:else}<li>No tags found.</li>{/each}
  </ul>
  {#if data.page.next_cursor}<a
      href={`?${new URLSearchParams({ search: data.search, archived: String(data.archived), after: data.page.next_cursor })}`}
      >Next page →</a
    >{/if}
</section>
<section>
  <h2>Create a tag</h2>
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
</section>
