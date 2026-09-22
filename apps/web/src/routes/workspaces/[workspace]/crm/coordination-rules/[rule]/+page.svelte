<script lang="ts">
  import RuleFields from '$lib/components/RuleFields.svelte';
  let { data, form } = $props();
</script>

<header>
  <a href={`/workspaces/${data.workspace.id}/crm/coordination-rules`}
    >← Status rules</a
  >
</header>
<h1>{data.rule.name}</h1>
<p>
  Changes affect future transitions. Existing jobs retain their original
  checklist revision and initiating user.
</p>
{#if form?.message}<p role="alert" class="error">{form.message}</p>
  <a href={`?reload=${Date.now()}`}>Discard draft and reload saved values</a
  >{/if}
<form method="POST">
  <input
    type="hidden"
    name="idempotency_key"
    value={form?.draft?.key ?? data.key}
  /><input
    type="hidden"
    name="expected_version"
    value={form?.draft?.expected_version ?? data.rule.version}
  /><RuleFields fields={form?.draft ?? data.fields} /><button>Save rule</button>
</form>
