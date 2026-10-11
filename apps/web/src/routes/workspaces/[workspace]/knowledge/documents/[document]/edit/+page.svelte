<script lang="ts">
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const root = $derived(
    `/workspaces/${data.workspace.id}/knowledge/documents/${data.document.id}`,
  );
  const values = $derived(form?.input ?? data.document);
  // Keep the version this draft started from: a conflict stays a conflict until
  // the author has seen the newer text and deliberately saves over it.
  const expected = $derived(
    form?.input?.expected_version ?? data.document.version,
  );
</script>

<svelte:head><title>Edit {data.document.title} · Looski</title></svelte:head>
<header><a href={root}>← Back to document</a></header>
<h1>Edit document</h1>
<p>
  Plain text with <code># headings</code>, <code>**bold**</code>,
  <code>*italic*</code>, <code>- lists</code> and
  <code>[links](https://example.com)</code>. Any HTML stays as literal text. To
  link another document, use the link shown on its page.
</p>
{#if form?.message}<p role="alert">{form.message}</p>{/if}
{#if form?.message && expected !== data.document.version}
  <section>
    <h2>Current text</h2>
    <p>
      Someone saved revision {data.document.revision_number} while you were editing.
      Your draft is below; copy what you need, then
      <a href={`${root}/edit`}>edit the current version</a>.
    </p>
    <pre>{data.document.body}</pre>
  </section>
{/if}
<form method="POST">
  <input type="hidden" name="idempotency_key" value={form?.key ?? data.key} />
  <input type="hidden" name="expected_version" value={expected} />
  <label
    >Kind<select name="kind">
      <option value="article" selected={values.kind !== 'sop'}>Article</option>
      <option value="sop" selected={values.kind === 'sop'}
        >Standard operating procedure</option
      >
    </select></label
  >
  <label
    >Title<input
      name="title"
      required
      maxlength="200"
      value={values.title}
    /></label
  >
  <label
    >Text<textarea name="body" rows="24" maxlength="100000" value={values.body}
    ></textarea></label
  >
  <button>Save revision</button>
</form>
