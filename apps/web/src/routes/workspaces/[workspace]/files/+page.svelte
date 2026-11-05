<script lang="ts">
  import { fileSize, shortDate } from '$lib/format';
  import RecordDrawer from '$lib/components/ui/RecordDrawer.svelte';
  import type { PageData, ActionData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
</script>

<svelte:head><title>Files · Looski Suite</title></svelte:head>

<div class="page-heading">
  <h1>Files</h1>
  <div class="page-actions">
    <RecordDrawer title="Upload file" error={Boolean(form?.message)}>
      {#if form?.message}<p role="alert">{form.message}</p>
        <p>
          Reselect the same file to retry. Your notes and retry key are
          retained.
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
        <label
          >File (up to 8 MiB)<input name="file" type="file" required /></label
        >
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
    </RecordDrawer>
  </div>
</div>

<section class="index-list">
  <form method="GET" class="inline">
    <label>Search filenames<input name="search" value={data.search} /></label
    ><label
      >Records<select name="archived" value={String(data.archived)}
        ><option value="false">Active</option><option value="true"
          >Archived</option
        ></select
      ></label
    ><button>Filter</button>
  </form>
  <div class="record-table">
    <table>
      <thead
        ><tr><th>Name</th><th>Type</th><th>Size</th><th>Updated</th></tr></thead
      ><tbody
        >{#each data.page.items as file}<tr
            ><td
              ><a href={`/workspaces/${data.workspace.id}/files/${file.id}`}
                >{file.filename}</a
              ></td
            ><td>{file.media_type}</td><td class="cell-date"
              >{fileSize(file.byte_length)}</td
            ><td class="cell-date">{shortDate(file.updated_at)}</td></tr
          >{:else}<tr><td colspan="4">No files match these filters.</td></tr
          >{/each}</tbody
      >
    </table>
  </div>
  {#if data.next}<a href={data.next}>Next page →</a>{/if}
</section>
