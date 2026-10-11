<script lang="ts">
  import { projectStatusLabel } from '$lib/project-labels';
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}/projects`);
</script>

<svelte:head><title>Projects · Looski</title></svelte:head>
<header><span>{data.workspace.name}</span></header>
<h1>Projects</h1>
<p>
  Organize delivery work on boards. Tasks stay in shared Tasks, so they keep
  their assignee, deadline and history wherever else they appear.
</p>
{#if form?.message && !form?.input}<p role="alert">{form.message}</p>{/if}
{#if data.page}
  <section>
    <div class="toolbar">
      <h2>{data.archived ? 'Archived projects' : 'Projects'}</h2>
      <a href={`${root}/my-work`}>My work</a>
      <a href={`${root}/recipes`}>Deal recipes</a>
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
          ><tr><th>Name</th><th>Status</th><th>Deadline</th><th>Member</th></tr
          ></thead
        >
        <tbody>
          {#each data.page.items as item}
            <tr>
              <td><a href={`${root}/${item.id}`}>{item.name}</a></td>
              <td>{projectStatusLabel(item.status)}</td>
              <td>{item.due_on ?? '—'}</td>
              <td>{item.is_member ? 'Yes' : 'No'}</td>
            </tr>
          {:else}<tr><td colspan="4">No projects found.</td></tr>{/each}
        </tbody>
      </table>
    </div>
    {#if data.page.next_cursor}<a
        href={`?${new URLSearchParams({ search: data.search, archived: String(data.archived), after: data.page.next_cursor })}`}
        >Next page →</a
      >{/if}
  </section>
  <section>
    <h2>Create a project</h2>
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
  <section>
    <h2>Application selection</h2>
    <p>Projects is {data.module.enabled ? 'on' : 'off'} for this workspace.</p>
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
  </section>
{/if}
