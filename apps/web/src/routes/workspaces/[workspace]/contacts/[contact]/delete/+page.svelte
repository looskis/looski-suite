<script lang="ts">
  import type { PageData, ActionData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
</script>

<a href={`/workspaces/${data.workspace.id}/contacts/${data.contact.id}`}
  >← Contact</a
>
<h1>Permanently delete {data.contact.display_name}</h1>
<p>
  This removes the contact’s name, notes and identifiers. It cannot be undone.
  Contacts with retained business references remain archived. Existing backups
  and independent business records are not erased.
</p>
{#if !data.contact.archived}<p>
    Archive this contact before requesting permanent deletion.
  </p>{:else}
  <p>Contact ID: <code>{data.contact.id}</code></p>
  {#if form?.message}<p role="alert">{form.message}</p>
    <a href="?reload=1">Discard draft and reload current version</a>{/if}
  <form method="POST">
    <input type="hidden" name="idempotency_key" value={form?.key ?? data.key} />
    <input
      type="hidden"
      name="expected_version"
      value={form?.draft?.expected_version ?? data.contact.version}
    />
    <label
      >Type the contact ID to confirm<input
        name="confirm_id"
        value={form?.draft?.confirm_id ?? ''}
        autocomplete="off"
        required
      /></label
    >
    <button>Permanently delete contact</button>
  </form>
{/if}
