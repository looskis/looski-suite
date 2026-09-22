<script lang="ts">
  import type { PageData, ActionData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
</script>

<svelte:head><title>Files · Looski Suite</title></svelte:head>
<header>
  <a href="/workspaces">← Workspaces</a><span>{data.workspace.name}</span>
</header>
<h1>Files</h1>
<p>
  Keep documents and other files in your workspace. Uploads retain their
  original content; edit the name or notes, or archive them when finished.
</p>
<section>
  <h2>Upload a file</h2>
  {#if form?.message}<p role="alert">{form.message}</p>
    <p>
      Reselect the same file to retry. Your notes and retry key are retained.
    </p>
    <a href={'/workspaces/' + data.workspace.id + '/files'}
      >Discard draft and start again</a
    >{/if}
  <form method="POST" enctype="multipart/form-data">
    <input
      type="hidden"
      name="idempotency_key"
      value={form?.draft.key ?? data.key}
    />
    <label>File (up to 8 MiB)<input name="file" type="file" required /></label>
    <label
      >Notes<textarea name="notes" maxlength="10000"
        >{form?.draft.notes ?? ''}</textarea
      ></label
    >
    <label
      >Responsible member ID (optional)<input
        name="owner_membership_id"
        value={form?.draft.owner_membership_id ?? ''}
        placeholder="Your membership by default"
      /></label
    >
    <button>Upload file</button>
  </form>
</section>
<section>
  <h2>Your files</h2>
  <form method="GET">
    <label>Search filenames<input name="search" value={data.search} /></label
    ><label
      >Records<select name="archived" value={String(data.archived)}
        ><option value="false">Active</option><option value="true"
          >Archived</option
        ></select
      ></label
    ><button>Filter</button>
  </form>
  {#each data.page.items as file}<p>
      <a href={'/workspaces/' + data.workspace.id + '/files/' + file.id}
        >{file.filename}</a
      >
      · {file.byte_length} bytes · {file.media_type}
    </p>{:else}<p>No files in this selection.</p>{/each}
  {#if data.next}<a href={data.next}>Next page →</a>{/if}
</section>
