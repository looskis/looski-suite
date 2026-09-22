<script lang="ts">
  import ActivityTypeFields from '$lib/components/ActivityTypeFields.svelte';
  let { data, form } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}/crm/activity-types`);
</script>

<header><a href={root}>← Activity types</a></header>
<h1>{data.type.name}</h1>
<p>Type ID: {data.type.id}</p>
{#if form?.message}<p role="alert" class="error">
    {form.message}
  </p>{/if}{#if form?.draft}<a href={`${root}/${data.type.id}`}
    >Discard draft and reload saved values</a
  >{/if}
<section>
  <h2>Type definition</h2>
  <form method="POST">
    <input
      type="hidden"
      name="idempotency_key"
      value={form?.draft?.key ?? data.key}
    /><input
      type="hidden"
      name="expected_version"
      value={form?.draft?.expected_version ?? data.type.version}
    /><ActivityTypeFields fields={form?.draft ?? data.type} /><button
      >Save type</button
    >
  </form>
</section>
