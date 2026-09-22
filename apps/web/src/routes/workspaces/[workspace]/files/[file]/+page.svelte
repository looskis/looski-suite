<script lang="ts">
  import type { PageData, ActionData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
  const draft = $derived(
    form?.draft.intent === 'edit'
      ? form.draft
      : {
          filename: data.file.filename,
          media_type: data.file.media_type,
          notes: data.file.notes,
          owner_membership_id: data.file.owner_membership_id,
          key: form?.draft.key ?? data.key,
          expected_version:
            form?.draft.expected_version ?? String(data.file.version),
          intent: form?.draft.intent ?? '',
        },
  );
  const archiveIntent = $derived(
    draft.intent === 'archive' || draft.intent === 'restore'
      ? draft.intent
      : data.file.archived
        ? 'restore'
        : 'archive',
  );
</script>

<svelte:head><title>{data.file.filename} · Files</title></svelte:head>
<header>
  <a href={'/workspaces/' + data.workspace.id + '/files'}>← Files</a><span
    >{data.workspace.name}</span
  >
</header>
<h1>{data.file.filename}</h1>
<p>
  {data.file.byte_length} bytes · {data.file.media_type} · Version {data.file
    .version}{data.file.archived ? ' · Archived' : ''}
</p>
<a
  class="button"
  href={'/workspaces/' +
    data.workspace.id +
    '/files/' +
    data.file.id +
    '/download'}>Download file</a
>
<details>
  <summary>Integrity information</summary>
  <p>SHA256: <code>{data.file.sha256}</code></p>
</details>
{#if form?.message}<p role="alert">{form.message}</p>
  <p>
    Your submitted version and retry key are retained. Review the draft before
    retrying.
  </p>
  <a href={'/workspaces/' + data.workspace.id + '/files/' + data.file.id}
    >Discard draft and reload current file</a
  >{/if}
{#if !data.file.archived || draft.intent === 'edit'}<section>
    <h2>File details</h2>
    <form method="POST">
      <input type="hidden" name="idempotency_key" value={draft.key} /><input
        type="hidden"
        name="expected_version"
        value={draft.expected_version}
      /><input type="hidden" name="intent" value="edit" />
      <label
        >Filename<input
          name="filename"
          value={draft.filename}
          maxlength="255"
          required
        /></label
      >
      <label
        >Media type<input
          name="media_type"
          value={draft.media_type}
          maxlength="127"
          required
        /></label
      >
      <label
        >Notes<textarea name="notes" maxlength="10000">{draft.notes}</textarea
        ></label
      >
      <label
        >Responsible member ID<input
          name="owner_membership_id"
          value={draft.owner_membership_id}
          required
        /></label
      ><button>Save details</button>
    </form>
  </section>{/if}
<section>
  <h2>{archiveIntent === 'restore' ? 'Restore file' : 'Archive file'}</h2>
  <p>
    Archiving retains the original content and keeps it available to authorized
    readers.
  </p>
  <form method="POST">
    <input type="hidden" name="idempotency_key" value={draft.key} /><input
      type="hidden"
      name="expected_version"
      value={draft.expected_version}
    /><input type="hidden" name="intent" value={archiveIntent} /><button
      >{archiveIntent === 'restore' ? 'Restore file' : 'Archive file'}</button
    >
  </form>
</section>

<style>
  code {
    overflow-wrap: anywhere;
  }
  .button {
    display: inline-block;
    margin: 12px 0;
  }
</style>
