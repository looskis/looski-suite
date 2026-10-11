<script lang="ts">
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}/forms`);
  const status = (item: NonNullable<PageData['page']>['items'][number]) =>
    item.archived
      ? 'Archived'
      : item.published_version_number === null ||
          item.published_version_number === undefined
        ? 'Draft'
        : item.accepting
          ? 'Accepting responses'
          : 'Closed';
</script>

<svelte:head><title>Forms · Looski</title></svelte:head>
<header><span>{data.workspace.name}</span></header>
<h1>Forms</h1>
<p>
  Collect answers through a public link or an embed. Publishing freezes a
  version; each submission stays tied to the version it answered.
</p>
{#if form?.message && !form?.input}<p role="alert">{form.message}</p>{/if}
{#if data.page}
  <section>
    <div class="toolbar">
      <h2>{data.archived ? 'Archived forms' : 'Forms'}</h2>
      <a href={data.archived ? '?' : '?archived=true'}
        >{data.archived ? 'Show active' : 'Show archived'}</a
      >
    </div>
    <form method="GET" class="inline">
      <label>Find by name<input name="search" value={data.search} /></label>
      <input type="hidden" name="archived" value={String(data.archived)} />
      <button class="secondary">Search</button>
    </form>
    <div class="table-wrap">
      <table>
        <thead
          ><tr
            ><th>Name</th><th>Status</th><th>Published version</th><th
              >Submissions</th
            ></tr
          ></thead
        >
        <tbody>
          {#each data.page.items as item}
            <tr>
              <td><a href={`${root}/${item.id}`}>{item.name}</a></td>
              <td>{status(item)}</td>
              <td>{item.published_version_number ?? '—'}</td>
              <td
                ><a href={`${root}/${item.id}/submissions`}
                  >{item.submission_count}</a
                ></td
              >
            </tr>
          {:else}<tr><td colspan="4">No forms found.</td></tr>{/each}
        </tbody>
      </table>
    </div>
    {#if data.page.next_cursor}<a
        href={`?${new URLSearchParams({ search: data.search, archived: String(data.archived), after: data.page.next_cursor })}`}
        >Next page →</a
      >{/if}
  </section>
  <section>
    <h2>Create a form</h2>
    {#if form?.message && form?.input}<p role="alert">{form.message}</p>{/if}
    <form method="POST" action="?/create">
      <input
        type="hidden"
        name="idempotency_key"
        value={form?.key ?? data.key}
      />
      <label
        >Name<input
          name="name"
          required
          maxlength="120"
          value={form?.input?.name ?? ''}
        /></label
      >
      <label
        >Description (shown above the questions)<textarea
          name="description"
          maxlength="2000"
          rows="2"
          value={form?.input?.description ?? ''}></textarea></label
      >
      <label
        >Confirmation message (shown after submitting)<textarea
          name="confirmation_message"
          maxlength="2000"
          rows="2"
          value={form?.input?.confirmation_message ?? ''}></textarea></label
      >
      <button>Create form</button>
    </form>
  </section>
{:else}
  <section>
    <h2>Forms is turned off</h2>
    <p>
      Existing forms and submissions are retained, and public links stay closed
      until Forms is turned on. Forms works whether or not CRM is on.
    </p>
  </section>
{/if}
{#if data.module}
  <section>
    <h2>Application selection</h2>
    <p>Forms is {data.module.enabled ? 'on' : 'off'} for this workspace.</p>
    <form method="POST" action="?/module">
      <input type="hidden" name="idempotency_key" value={data.key} />
      <input
        type="hidden"
        name="expected_version"
        value={data.module.version}
      />
      <input
        type="hidden"
        name="enabled"
        value={String(!data.module.enabled)}
      />
      <button class={data.module.enabled ? 'secondary' : ''}
        >{data.module.enabled ? 'Turn off Forms' : 'Turn on Forms'}</button
      >
    </form>
  </section>
{/if}
