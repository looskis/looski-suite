<script lang="ts">
  import ContactFields from '$lib/components/ContactFields.svelte';
  import type { PageData, ActionData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const nextPage = () =>
    `?${new URLSearchParams({ search: data.search, archived: String(data.archived), after: data.page.next_cursor ?? '' })}`;
</script>

<svelte:head
  ><title>Contacts · {data.workspace.name} · Looski</title></svelte:head
>
<header>
  <a href="/workspaces">← Workspaces</a><span>{data.workspace.name}</span>
  <form method="POST" action="/logout">
    <button class="secondary">Sign out</button>
  </form>
</header>
<h1>People behind the business.</h1>
<p class="muted">
  Keep relationships in one place. Contacts work independently of your sales
  pipeline.
</p>
<section aria-label="Contacts">
  <div class="toolbar">
    <h2>{data.archived ? 'Archived contacts' : 'Contacts'}</h2>
    <a href={data.archived ? '?' : '?archived=true'}
      >{data.archived ? 'Show active contacts' : 'Show archived contacts'}</a
    >
  </div>
  <form method="GET" class="inline">
    <label
      >Find by name<input
        name="search"
        value={data.search}
        placeholder="Search contacts"
      /></label
    ><input
      type="hidden"
      name="archived"
      value={String(data.archived)}
    /><button>Search</button>
  </form>
  <div class="table-wrap">
    <table>
      <thead><tr><th>Name</th><th>Email</th><th>Phone</th></tr></thead><tbody>
        {#each data.page.items as contact (contact.id)}
          <tr
            ><td
              ><a
                href={`/workspaces/${data.workspace.id}/contacts/${contact.id}`}
                >{contact.display_name}</a
              ></td
            ><td
              >{contact.identifiers
                .filter((i) => i.kind === 'email')
                .map((i) => i.value)
                .join(', ') || '—'}</td
            ><td
              >{contact.identifiers
                .filter((i) => i.kind === 'phone')
                .map((i) => i.value)
                .join(', ') || '—'}</td
            ></tr
          >
        {:else}<tr
            ><td colspan="3"
              >{data.search
                ? 'No matching contacts.'
                : 'No contacts here yet.'}</td
            ></tr
          >{/each}
      </tbody>
    </table>
  </div>
  {#if data.page.next_cursor}<p><a href={nextPage()}>Next page →</a></p>{/if}
</section>
<section aria-labelledby="create-heading">
  <h2 id="create-heading">Add a contact</h2>
  {#if form?.message}<p role="alert" class="error">{form.message}</p>{/if}
  <form method="POST" action="?/create">
    <input
      type="hidden"
      name="idempotency_key"
      value={form?.idempotencyKey ?? data.idempotencyKey}
    /><ContactFields contact={form?.input} /><button>Create contact</button>
  </form>
</section>
