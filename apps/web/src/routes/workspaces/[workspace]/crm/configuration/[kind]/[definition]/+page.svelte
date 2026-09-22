<script lang="ts">
  import CrmDefinitionFields from '$lib/components/CrmDefinitionFields.svelte';
  let { data, form } = $props();
</script>

<header>
  <a href={`/workspaces/${data.workspace.id}/crm/configuration/${data.kind}`}
    >← Definitions</a
  ><span>Version {data.definition.version}</span>
</header>
<h1>{data.definition.name}</h1>
{#if form?.message}<p class="error" role="alert">{form.message}</p>{/if}
<section>
  <form method="POST">
    <input
      type="hidden"
      name="idempotency_key"
      value={form?.key ?? data.key}
    /><input
      type="hidden"
      name="expected_version"
      value={form?.draft?.expected_version ?? data.definition.version}
    /><CrmDefinitionFields
      kind={data.kind}
      draft={form?.draft}
      definition={data.definition}
      parents={[]}
    /><button>Save definition</button>
  </form>
</section>

{#if data.kind === 'pipelines' || data.kind === 'categories'}<p>
    <a
      href={`/workspaces/${data.workspace.id}/crm/${data.kind === 'pipelines' ? 'opportunities' : 'deals'}?parent=${data.definition.id}`}
      >Use this workflow →</a
    >
  </p>{/if}
