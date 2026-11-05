<script lang="ts">
  import Icon from '$lib/components/Icon.svelte';
  import { shortDate } from '$lib/format';
  import RecordDrawer from '$lib/components/ui/RecordDrawer.svelte';
  import { projectStatusLabel } from '$lib/project-labels';
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}/projects`);
</script>

<svelte:head><title>Projects · Looski</title></svelte:head>

<div class="page-heading">
  <h1>Projects</h1>
  <div class="page-actions">
    {#if data.page}<RecordDrawer
        title="New project"
        error={Boolean(form?.input)}
      >
        {#if form?.message && form?.input}<p role="alert">
            {form.message}
          </p>{/if}
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
            >Description<textarea
              name="description"
              maxlength="4000"
              rows="2"
              value={form?.input?.description ?? ''}></textarea></label
          >
          <label
            >Deadline<input
              name="due_on"
              type="date"
              value={form?.input?.due_on ?? ''}
            /></label
          >
          <button>Create project</button>
        </form>
      </RecordDrawer>{/if}
  </div>
</div>
{#if form?.message && !form?.input}<p role="alert">{form.message}</p>{/if}
{#if data.page}
  <section class="index-list">
    <div class="toolbar">
      <nav class="view-tabs" aria-label="Projects views">
        <a href="?" aria-current={!data.archived ? 'page' : undefined}
          >All projects</a
        ><a
          href="?archived=true"
          aria-current={data.archived ? 'page' : undefined}>Archived</a
        >
      </nav>
      <a href={`${root}/my-work`}>My work</a>
      <a href={`${root}/recipes`}>Deal recipes</a>
    </div>
    <form method="GET" class="inline">
      <label>Find by name<input name="search" value={data.search} /></label>
      <input type="hidden" name="archived" value={String(data.archived)} />
      <button class="secondary">Search</button>
    </form>
    <div class="project-grid">
      {#each data.page.items as item}<a
          class="project-card"
          href={`${root}/${item.id}`}
          ><div class="project-card-top">
            <span class="project-icon" aria-hidden="true"
              ><Icon name="board" size={18} /></span
            ><span class="status-badge">{projectStatusLabel(item.status)}</span>
          </div>
          <h2>{item.name}</h2>
          <p>{item.description || 'No description'}</p>
          <div class="project-card-footer">
            <span>{item.due_on ? shortDate(item.due_on) : 'No deadline'}</span
            ><span>{item.is_member ? 'Member' : 'View project'} ↗</span>
          </div></a
        >{:else}<div class="empty-state">
          <strong>No projects found</strong>
          <p>Create a project to organize your team's work.</p>
        </div>{/each}
    </div>
    {#if data.page.next_cursor}<a
        href={`?${new URLSearchParams({ search: data.search, archived: String(data.archived), after: data.page.next_cursor })}`}
        >Next page →</a
      >{/if}
  </section>
{:else}
  <section>
    <h2>Projects is turned off</h2>
    <p>
      Existing projects, boards and comments are retained. Shared Tasks keeps
      working, and CRM does not depend on Projects.
    </p>
  </section>
{/if}
{#if data.module}
  <details class="creation-panel" open={Boolean(form?.message)}>
    <summary>Application settings</summary>
    <div class="creation-content">
      <p>
        Projects is {data.module.enabled ? 'on' : 'off'} for this workspace.
      </p>
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
          >{data.module.enabled
            ? 'Turn off Projects'
            : 'Turn on Projects'}</button
        >
      </form>
    </div>
  </details>
{/if}

<style>
  .project-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(270px, 1fr));
    gap: 16px;
    margin: 20px 0;
  }
  .project-card {
    display: flex;
    flex-direction: column;
    padding: 20px;
    border: 1px solid var(--border-default);
    border-radius: 10px;
    background: var(--surface-panel);
    text-decoration: none;
    color: var(--text-primary);
    transition:
      box-shadow 0.15s,
      border-color 0.15s;
  }
  .project-card:hover {
    box-shadow: 0 4px 16px #0000000a;
    border-color: var(--text-secondary);
  }
  .project-card-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .project-icon {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: 9px;
    background: var(--surface-hover);
    font-size: 20px;
  }
  .project-card h2 {
    font-size: 15px;
    margin: 20px 0 8px;
    overflow-wrap: anywhere;
  }
  .project-card p {
    color: var(--text-secondary);
    font-size: 12px;
    line-height: 1.5;
    margin: 0 0 24px;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .project-card-footer {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    margin-top: auto;
    padding-top: 14px;
    border-top: 1px solid var(--border-default);
    font-size: 10px;
    color: var(--text-secondary);
  }
</style>
