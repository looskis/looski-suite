<script lang="ts">
  let { data } = $props();
</script>

<header><a href="/workspaces">← Workspace</a></header>
<h1>Background work</h1>
<p>
  Blocked or exhausted work requires an explicit resume. Resuming retains the
  initiating user and rechecks their current permissions.
</p>
<ul>
  {#each data.page.items as job}<li>
      <a href={`/workspaces/${data.workspace.id}/jobs/${job.id}`}
        >{job.kind} · {job.state}</a
      >
      · {job.created_at}{#if job.error_code}
        · {job.error_code.replaceAll('_', ' ')}{/if}
    </li>{:else}<li>No scheduled work.</li>{/each}
</ul>
{#if data.page.next_cursor}<a href={`?after=${data.page.next_cursor}`}
    >More jobs →</a
  >{/if}
