<script lang="ts">
  import OrganizationFields from '$lib/components/OrganizationFields.svelte';
  import type { PageData, ActionData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const nextPage = () =>
    `?${new URLSearchParams({ search: data.search, archived: String(data.archived), after: data.page.next_cursor ?? '' })}`;
</script>

<svelte:head
  ><title>Organizations · {data.workspace.name} · Looski</title></svelte:head
>
<header>
  <a href="/workspaces">← Workspaces</a><span>{data.workspace.name}</span><a
    href={`/workspaces/${data.workspace.id}/contacts`}>Contacts</a
  >
  <form method="POST" action="/logout">
    <button class="secondary">Sign out</button>
  </form>
</header>
<h1>Organizations you work with.</h1>
<p class="muted">
  Connect companies to the people you know and keep their relationship history
  together.
</p>
<section aria-label="Organizations">
  <div class="toolbar">
    <h2>{data.archived ? 'Archived organizations' : 'Organizations'}</h2>
    <a href={data.archived ? '?' : '?archived=true'}
      >{data.archived
        ? 'Show active organizations'
        : 'Show archived organizations'}</a
    >
  </div>
  <form method="GET" class="inline">
    <label
      >Find by name<input
        name="search"
        value={data.search}
        placeholder="Search organizations"
      /></label
    ><input
      type="hidden"
      name="archived"
      value={String(data.archived)}
    /><button>Search</button>
  </form>
  <div class="table-wrap">
    <table>
      <thead><tr><th>Name</th><th>Website</th></tr></thead><tbody
        >{#each data.page.items as organization (organization.id)}<tr
            ><td
              ><a
                href={`/workspaces/${data.workspace.id}/organizations/${organization.id}`}
                >{organization.name}</a
              ></td
            ><td>{organization.website_url ?? '—'}</td></tr
          >{:else}<tr
            ><td colspan="2"
              >{data.search
                ? 'No matching organizations.'
                : 'No organizations here yet.'}</td
            ></tr
          >{/each}</tbody
      >
    </table>
  </div>
  {#if data.page.next_cursor}<p><a href={nextPage()}>Next page →</a></p>{/if}
</section>
<section aria-labelledby="create-heading">
  <h2 id="create-heading">Add an organization</h2>
  {#if form?.message}<p role="alert" class="error">{form.message}</p>{/if}
  <form method="POST" action="?/create">
    <input
      type="hidden"
      name="idempotency_key"
      value={form?.idempotencyKey ?? data.idempotencyKey}
    /><OrganizationFields organization={form?.input} /><button
      >Create organization</button
    >
  </form>
</section>
