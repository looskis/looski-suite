<script lang="ts">
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const root = $derived(`/workspaces/${data.workspace.id}/knowledge`);
  const chosen = $derived(form?.input?.collection_id ?? data.selected);
</script>

<svelte:head><title>New document · Looski</title></svelte:head>
<header><a href={root}>← Knowledge base</a></header>
<h1>New document</h1>
<p>
  Plain text with <code># headings</code>, <code>**bold**</code>,
  <code>*italic*</code>, <code>- lists</code> and
  <code>[links](https://example.com)</code>. Any HTML stays as literal text. To
  link another document, use the link shown on its page.
</p>
{#if form?.message}<p role="alert">{form.message}</p>{/if}
<form method="POST">
  <input type="hidden" name="idempotency_key" value={form?.key ?? data.key} />
  <label
    >Collection<select name="collection_id" required>
      {#each data.collections as item (item.id)}<option
          value={item.id}
          selected={item.id === chosen}
          >{item.name}{item.restricted ? ' (restricted)' : ''}</option
        >{/each}
    </select></label
  >
  <label
    >Kind<select name="kind">
      <option value="article" selected={form?.input?.kind !== 'sop'}
        >Article</option
      >
      <option value="sop" selected={form?.input?.kind === 'sop'}
        >Standard operating procedure</option
      >
    </select></label
  >
  <label
    >Title<input
      name="title"
      required
      maxlength="200"
      value={form?.input?.title ?? ''}
    /></label
  >
  <label
    >Text<textarea
      name="body"
      rows="18"
      maxlength="100000"
      value={form?.input?.body ?? ''}></textarea></label
  >
  <button>Create document</button>
</form>
