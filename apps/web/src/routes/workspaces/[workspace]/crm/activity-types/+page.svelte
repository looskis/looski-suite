<script lang="ts">
  import ActivityTypeFields from '$lib/components/ActivityTypeFields.svelte';
  let { data, form } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}/crm/activity-types`);
</script>

<header><a href={`/workspaces/${data.workspace.id}/crm`}>← CRM</a></header>
<h1>Activity types</h1>
<p>
  Choose the names and supported behavior that fit your business. Retiring a
  type preserves its activities and history.
</p>
{#if form?.message}<p role="alert" class="error">{form.message}</p>{/if}
<a href={`?archived=${!data.archived}`}
  >{data.archived ? 'Show active' : 'Show archived'}</a
>
<section>
  <h2>{data.archived ? 'Archived' : 'Active'} types</h2>
  <ul>
    {#each data.page.items as type}<li>
        <a href={`${root}/${type.id}`}>{type.name}</a> · {type.behavior} · Order {type.position}
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
    /><ActivityTypeFields
      fields={form?.draft ?? {
        name: '',
        behavior: 'note',
        position: 0,
        archived: false,
      }}
    /><button>Create type</button>
  </form>
</section>
