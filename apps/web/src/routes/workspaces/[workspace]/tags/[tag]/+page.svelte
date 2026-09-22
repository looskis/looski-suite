<script lang="ts">
  import type { PageData, ActionData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const input = $derived(form?.input ?? data.tag);
</script>

<svelte:head><title>{data.tag.name} · Tags · Looski</title></svelte:head><a
  href={`/workspaces/${data.workspace.id}/tags`}>← Tags</a
>
<h1>{data.tag.name}</h1>
<p>
  Archiving keeps current assignments and prevents new ones. Restore the tag to
  use it again.
</p>
<section>
  {#if form?.message}<p role="alert" class="error">{form.message}</p>{/if}
  <form method="POST">
    <input
      type="hidden"
      name="idempotency_key"
      value={form?.key ?? data.key}
    /><input
      type="hidden"
      name="expected_version"
      value={form?.input?.expected_version ?? data.tag.version}
    /><label
      >Name<input
        name="name"
        required
        maxlength="80"
        value={input.name}
      /></label
    ><label>Color<input name="color" value={input.color ?? ''} /></label><label
      class="check"
      ><input
        type="checkbox"
        name="archived"
        checked={input.archived}
      />Archived</label
    ><button>Save tag</button>
  </form>
</section>
