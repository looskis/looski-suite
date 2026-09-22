<script lang="ts">
  import CrmDefinitionFields from '$lib/components/CrmDefinitionFields.svelte';
  let { data, form } = $props();
</script>

<header>
  <a href={`/workspaces/${data.workspace.id}/crm`}>← CRM</a><a
    href={`?archived=${!data.archived}`}
    >{data.archived ? 'View active' : 'View archived'}</a
  >
</header>
<h1>{data.title}</h1>
{#if form?.message}<p class="error" role="alert">{form.message}</p>{/if}
<section>
  <div class="table-wrap">
    <table>
      <thead><tr><th>Name</th><th>Order</th><th>Outcome</th></tr></thead><tbody
        >{#each data.page.items as definition}<tr
            ><td
              ><a
                href={`/workspaces/${data.workspace.id}/crm/configuration/${data.kind}/${definition.id}`}
                >{definition.name}</a
              ></td
            ><td>{definition.position}</td><td>{definition.outcome ?? '—'}</td
            ></tr
          >{/each}</tbody
      >
    </table>
  </div>
  {#if data.page.next_cursor}<p>
      <a href={`?after=${data.page.next_cursor}&archived=${data.archived}`}
        >More definitions →</a
      >
    </p>{/if}
</section>
<section>
  <h2>Create definition</h2>
  <form method="POST">
    <input
      type="hidden"
      name="idempotency_key"
      value={form?.key ?? data.key}
    /><CrmDefinitionFields
      kind={data.kind}
      draft={form?.draft}
      parents={data.parents}
    /><button>Create</button>
  </form>
  {#if data.parents_next}<p>
      <a href={`?parents_after=${data.parents_next}`}>More parent choices →</a>
    </p>{/if}
</section>
