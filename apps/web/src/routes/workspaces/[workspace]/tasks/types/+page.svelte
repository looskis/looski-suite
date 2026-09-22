<script lang="ts">
  import TaskTypeFields from '$lib/components/TaskTypeFields.svelte';
  let { data, form } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}/tasks/types`);
</script>

<header><a href={`/workspaces/${data.workspace.id}/tasks`}>← Tasks</a></header>
<h1>Task types</h1>
<p>
  Choose the names and classification that fit your business. Retiring a type
  preserves its tasks and history.
</p>
{#if form?.message}<p role="alert" class="error">{form.message}</p>{/if}
<a href={`?archived=${!data.archived}`}
  >{data.archived ? 'Show active' : 'Show archived'}</a
>
<section>
  <h2>{data.archived ? 'Archived' : 'Active'} types</h2>
  <ul>
    {#each data.page.items as type}<li>
        <a href={`${root}/${type.id}`}>{type.name}</a> · Order {type.position}
      </li>{:else}<li>No types yet.</li>{/each}
  </ul>
  {#if data.page.next_cursor}<a
      href={`?archived=${data.archived}&after=${data.page.next_cursor}`}
      >More types →</a
    >{/if}
</section>
<section>
  <h2>Create a type</h2>
  <form method="POST">
    <input
      type="hidden"
      name="idempotency_key"
      value={form?.draft?.key ?? data.key}
    /><TaskTypeFields
      fields={form?.draft ?? {
        name: '',

        position: 0,
        archived: false,
      }}
    /><button>Create type</button>
  </form>
</section>
