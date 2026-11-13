<script lang="ts">
  import RecordDrawer from '$lib/components/ui/RecordDrawer.svelte';
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}/cms`);
  const nextQuery = $derived(
    data.page?.next_cursor
      ? `?${new URLSearchParams({ search: data.search, archived: String(data.archived), type: data.typeId, review: data.reviewState, after: data.page.next_cursor })}`
      : '',
  );
</script>

<svelte:head><title>Content · Looski</title></svelte:head>

<div class="page-heading">
  <h1>{data.writing?.contributor_only ? 'My articles' : 'Content'}</h1>
  <div class="page-actions">
    {#if data.page && data.writing?.can_edit}<RecordDrawer
        title="New entry"
        error={Boolean(form?.input)}
      >
        {#if form?.message && form?.input}<p role="alert">
            {form.message}
          </p>{/if}
        {#if data.types.length}
          <form method="POST" action="?/create">
            <input
              type="hidden"
              name="idempotency_key"
              value={form?.key ?? data.key}
            />
            <label
              >Content type<select name="type_id" required>
                {#each data.types as type}
                  <option value={type.id}>{type.name}</option>
                {/each}
              </select></label
            >
            <label
              >Title<input
                name="title"
                required
                maxlength="200"
                value={form?.input?.title ?? ''}
              /></label
            >
            <label
              >Address (made from the title if left empty)<input
                name="slug"
                maxlength="120"
                value={form?.input?.slug ?? ''}
              /></label
            >
            <button>Create entry</button>
          </form>
        {:else}
          <p>
            <a href={`${root}/types`}>Create a content type</a> first; it sets which
            fields an entry has.
          </p>
        {/if}
      </RecordDrawer>{/if}
  </div>
</div>
{#if form?.message && !form?.input}<p role="alert">{form.message}</p>{/if}
{#if data.page}
  <section class="index-list">
    <div class="toolbar">
      <nav class="view-tabs" aria-label="Entries views">
        <a
          href="?"
          aria-current={!data.archived && !data.reviewState
            ? 'page'
            : undefined}>All entries</a
        ><a
          href="?archived=true"
          aria-current={data.archived ? 'page' : undefined}>Archived</a
        >
        {#if data.writing?.can_publish}<a
            href="?review=submitted"
            aria-current={data.reviewState === 'submitted' ? 'page' : undefined}
            >Review queue</a
          >{/if}
      </nav>
    </div>
    <form method="GET" class="inline">
      <label>Find by title<input name="search" value={data.search} /></label>
      <label
        >Content type<select name="type">
          <option value="">All</option>
          {#each data.types as type}
            <option value={type.id} selected={type.id === data.typeId}
              >{type.name}</option
            >
          {/each}
        </select></label
      >
      <input type="hidden" name="archived" value={String(data.archived)} />
      <input type="hidden" name="review" value={data.reviewState} />
      <button class="secondary">Search</button>
    </form>
    <div class="record-table">
      <table>
        <thead
          ><tr
            ><th>Title</th><th>Type</th><th>Status</th><th>Public page</th></tr
          ></thead
        >
        <tbody>
          {#each data.page.items as item}
            <tr>
              <td><a href={`${root}/entries/${item.id}`}>{item.title}</a></td>
              <td>{item.type_name}</td>
              <td>
                {#if item.archived}Archived
                {:else if item.contributor_id && item.review_state !== 'published'}{item.review_state.replaceAll(
                    '_',
                    ' ',
                  )}
                {:else if item.published_revision_number === null || item.published_revision_number === undefined}Draft
                {:else if item.unpublished_changes}Published, with unpublished
                  changes
                {:else}Published{/if}
              </td>
              <td>
                {#if item.published_slug}<a
                    href={`/c/${item.public_key}/${item.published_slug}`}
                    >/{item.published_slug}</a
                  >{:else}—{/if}
              </td>
            </tr>
          {:else}<tr><td colspan="4">No entries found.</td></tr>{/each}
        </tbody>
      </table>
    </div>
    {#if nextQuery}<a href={nextQuery}>Next page →</a>{/if}
  </section>
{:else}
  <section>
    {#if data.pending}<h2>Waiting for collection access</h2>
      <p>
        An editor needs to approve your collection before you can start writing.
      </p>{:else}
      <h2>Content is turned off</h2>
      <p>
        Existing content is retained, and public pages stay closed until Content
        is turned on. It works whether or not Forms or CRM are on.
      </p>{/if}
  </section>
{/if}
{#if data.module && !data.writing?.contributor_only}
  <section>
    <h2>{data.module.enabled ? 'Content is enabled' : 'Enable Content'}</h2>
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
        value={data.module.enabled ? 'false' : 'true'}
      />
      <button class={data.module.enabled ? 'secondary' : ''}
        >{data.module.enabled ? 'Disable Content' : 'Enable Content'}</button
      >
    </form>
  </section>
{/if}
