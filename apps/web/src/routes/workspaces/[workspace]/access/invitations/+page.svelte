<script lang="ts">
  import type { ActionData, PageData } from './$types';
  let { data, form }: { data: PageData; form: ActionData } = $props();
</script>

<a href={`/workspaces/${data.workspace.id}/access`}>← Workspace access</a>
<h1>Staff invitations</h1>
<p>
  Create an expiring invitation for a specific email address. The recipient must
  verify that address before joining.
</p>
{#if form?.message}<p role="alert" class="error">{form.message}</p>{/if}
<section>
  <h2>Choose roles</h2>
  <form method="GET">
    {#each data.selected.filter((r) => !data.roles.items.some((item) => item.id === r.id)) as role}<label
        ><input
          type="checkbox"
          name="role_ids"
          value={role.id}
          checked
        />{role.name}{role.archived ? ' (archived)' : ''}</label
      >{/each}
    {#each data.roles.items.filter((r) => !r.archived) as role}<label
        ><input
          type="checkbox"
          name="role_ids"
          value={role.id}
          checked={data.selected.some((r) => r.id === role.id)}
        />{role.name}</label
      >{/each}
    <p>
      <button>Use these roles</button>
      {#if data.roles.next_cursor}<button
          name="roles_after"
          value={data.roles.next_cursor}
          >Keep selection and show more roles</button
        >{/if}
    </p>
  </form>
</section>
<section>
  <h2>Create invitation</h2>
  <p>
    Selected roles: {data.selected.map((r) => r.name).join(', ') ||
      'Choose roles above first'}
  </p>
  <form method="POST">
    <input type="hidden" name="idempotency_key" value={form?.key ?? data.key} />
    {#each form?.draft.role_ids ?? data.selected.map((r) => r.id) as id}<input
        type="hidden"
        name="role_ids"
        value={id}
      />{/each}
    <label
      >Email <input
        name="email"
        type="email"
        maxlength="254"
        required
        value={form?.draft.email ?? ''}
      /></label
    >
    <label
      >Expires in hours <input
        name="expires_in_hours"
        type="number"
        min="1"
        max="168"
        required
        value={form?.draft.expires_in_hours ?? 72}
      /></label
    >
    <button>Create invitation</button>
  </form>
</section>
<section>
  <h2>Invitation history</h2>
  <table>
    <thead><tr><th>Recipient</th><th>State</th><th>Expires</th></tr></thead
    ><tbody>
      {#each data.invitations.items as invitation}<tr
          ><td
            ><a
              href={`/workspaces/${data.workspace.id}/access/invitations/${invitation.id}`}
              >{invitation.email}</a
            ></td
          ><td>{invitation.state}</td><td>{invitation.expires_at}</td></tr
        >{/each}
    </tbody>
  </table>
  {#if data.invitations.next_cursor}<a
      href={`?after=${data.invitations.next_cursor}`}>More invitations →</a
    >{/if}
</section>
